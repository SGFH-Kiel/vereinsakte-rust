# arbeitsverwaltung Specification

## Purpose

Macht Arbeitsstunden im Web erfassbar und prüfbar: Arbeitseinsätze und ihre Bestätigung, Arbeitstermine mit Anmeldungen, Aufgaben sowie Stundenüberträge und finanzielle Abgeltung.

## Requirements

### Requirement: Arbeitseinsatz erfassen
Ein Mitglied SHALL für sich einen Arbeitseinsatz mit Datum, Gegenstand (Boot oder Verein), Beschreibung und entweder Stunden oder Zeitraum erfassen können, optional mit Bezug zu einem Arbeitstermin oder einer Aufgabe. Unplausible Angaben (beide Umfangsformen, fehlender Gegenstand, Boot ohne Bezug) SHALL nach der Spec `arbeitsstunden` abgelehnt werden.

#### Scenario: Einsatz erfassen
- **WHEN** Clara „Segel der Filou geflickt“, 3 Stunden, Gegenstand Filou erfasst
- **THEN** erscheint der Einsatz unbestätigt in ihrer Stundenliste

#### Scenario: Beide Umfangsformen
- **WHEN** zugleich Stunden und Zeitraum angegeben werden
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

### Requirement: Eigene unbestätigte Einsätze ändern
Ein Mitglied SHALL seine eigenen unbestätigten Einsätze ändern und löschen können. Bestätigte Einsätze SHALL nur die Berechtigten nach der Spec `aemter` ändern; eine Änderung SHALL die Bestätigung aufheben.

#### Scenario: Bestätigten Einsatz bearbeiten
- **WHEN** Clara einen bereits bestätigten Einsatz ändern will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Einsätze bestätigen
Berechtigte nach der Spec `aemter` SHALL offene Einsätze ihres Zuständigkeitsbereichs gesammelt sehen und bestätigen können. Niemand SHALL den eigenen Einsatz bestätigen können; bestätigte Einsätze SHALL den Bestätigenden und den Zeitpunkt zeigen.

#### Scenario: Bootswart bestätigt
- **WHEN** Ben als Bootswart der Harald Blauzahn Claras Einsatz an diesem Boot bestätigt
- **THEN** zählt der Einsatz zu den anrechenbaren Stunden

#### Scenario: Eigenen Einsatz bestätigen
- **WHEN** Ben seinen eigenen Einsatz bestätigen will
- **THEN** wird dies abgelehnt und der Einsatz bleibt unbestätigt

### Requirement: Arbeitstermine und Anmeldungen
Der Vorstand und der Oberbootswart SHALL Arbeitstermine mit Datum, Beschreibung und Platzzahl anlegen und ändern können. Jedes Mitglied SHALL sich zu einem Termin an- und abmelden können; die Anmeldung SHALL keine Stunden gutschreiben, solange kein Einsatz erfasst ist. Ein ausgebuchter Termin SHALL keine weitere Anmeldung annehmen.

#### Scenario: Anmelden
- **WHEN** Anna sich zum Arbeitstermin „Frühjahrsputz“ anmeldet
- **THEN** erscheint sie in der Teilnehmerliste und auf ihrer Startseite

#### Scenario: Ausgebucht
- **WHEN** alle Plätze vergeben sind und ein weiteres Mitglied sich anmelden will
- **THEN** zeigt die Seite „ausgebucht“ und es entsteht keine Anmeldung

### Requirement: Aufgaben
Amtsträger SHALL Aufgaben anlegen und Zuständige zuweisen können. Ein Zuständiger SHALL den Status seiner Aufgabe ändern können; jedes Mitglied SHALL die Aufgaben mit Zuständigen und Status lesen. Zu einer Aufgabe erfasste Einsätze SHALL die Stunden der Aufgabe zeigen.

#### Scenario: Status setzen
- **WHEN** der Zuständige eine Aufgabe auf „erledigt“ setzt
- **THEN** zeigt die Liste „erledigt“ mit Zeitpunkt

#### Scenario: Fremde Aufgabe
- **WHEN** ein Mitglied ohne Zuständigkeit und ohne Amt den Status ändern will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Übertrag und Abgeltung
Der Vorstand SHALL Stundenüberträge zwischen Mitgliedern erfassen und eine finanzielle Abgeltung genehmigen können. Eine Abgeltung SHALL die Forderung nach der Spec `arbeitsstunden` im Konto erzeugen und den Betrag aus dem Stundensatz der Saison vorschlagen.

#### Scenario: Abgeltung
- **WHEN** Eva für Clara 5 Stunden bei einem Stundensatz von 15 € genehmigt
- **THEN** entsteht eine Forderung über 75,00 € auf Claras Konto

#### Scenario: Übertrag an sich selbst
- **WHEN** ein Übertrag mit demselben Mitglied als Geber und Empfänger eingegeben wird
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

### Requirement: Stundenübersicht
Jedes Mitglied SHALL seine anrechenbaren und Soll-Stunden je Jahr mit allen Einsätzen sehen; der Vorstand SHALL die Stunden aller Mitglieder je Jahr mit Filter „Rückstand“ sehen.

#### Scenario: Rückstandsliste
- **WHEN** der Vorstand die Stundenübersicht mit Filter „Rückstand“ öffnet
- **THEN** erscheint Clara mit „es fehlen 4,5 Arbeitsstunden“

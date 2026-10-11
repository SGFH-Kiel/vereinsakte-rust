# rechte Specification

## Purpose

Legt fest, welche Daten der Vereinsakte ein angemeldetes Mitglied im Web lesen und ändern darf, abgeleitet aus seinen Ämtern zum aktuellen Zeitpunkt, und wie das System bei fehlender Berechtigung reagiert.

## Requirements

### Requirement: Rollen aus den aktuellen Ämtern
Die Rolle eines angemeldeten Mitglieds SHALL sich bei jeder Anfrage aus den Ämtern zum aktuellen Zeitpunkt ergeben: jedes aktive Mitglied, Vorstand, Kassenwart, Schriftwart, Oberbootswart, Revisor sowie Bootswart und Abnehmer je Boot. Eine Rolle SHALL nicht in der Sitzung gespeichert werden.

#### Scenario: Amtswechsel wirkt sofort
- **WHEN** Ben als Bootswart abgibt, während er angemeldet ist
- **THEN** darf er bei seiner nächsten Anfrage die Boote dieses Amtes nicht mehr sperren

#### Scenario: Mehrere Ämter
- **WHEN** ein Mitglied zugleich Kassenwart und Bootswart der Filou ist
- **THEN** gelten die Rechte beider Ämter

### Requirement: Rechtematrix
Die Berechtigung SHALL je Bereich nach der Rechtematrix dieser Spec bestimmt werden. Alle aktiven Mitglieder lesen Verzeichnis, Boote (ohne interne Vermerke), Arbeitstermine, Aufgaben, Ämter und Reservierungskalender. Fremde Konten, Forderungen und Arbeitsstunden SHALL nur lesen, wer in den folgenden Anforderungen dafür genannt ist.

#### Scenario: Mitglied ohne Amt
- **WHEN** Greta ohne Amt das Mitgliederverzeichnis, die Boote und den Reservierungskalender öffnet
- **THEN** sieht sie alle drei, aber weder fremde Konten noch fremde Kontaktdaten

### Requirement: Mitglied und eigene Daten
Jedes aktive Mitglied SHALL seine eigenen Stammdaten, Segelerlaubnis, Arbeitsstunden und sein Konto lesen und seine Kontaktdaten (Anschrift, Telefonnummern, Kontakt-E-Mail) sowie seine eigenen Segelscheine und Funklizenzen ändern dürfen. Es SHALL eigene Arbeitseinsätze erfassen und solange unbestätigt ändern oder löschen, sich zu Arbeitsterminen an- und abmelden und eigene Reservierungen anlegen und stornieren dürfen.

#### Scenario: Eigene Kontaktdaten
- **WHEN** Anna ihre Telefonnummer ändert
- **THEN** wird die Änderung gespeichert

#### Scenario: Eigenen Schein erfassen
- **WHEN** Frank einen eigenen SKS-Schein erfasst
- **THEN** wird er ohne Nachweis und ohne Freigabe gespeichert

#### Scenario: Fremde Kontaktdaten
- **WHEN** Greta die Kontaktdaten von Clara ändern will
- **THEN** wird die Aktion mit 403 abgelehnt und nichts gespeichert

### Requirement: Vorstand und Schriftwart
Mitglieder aufnehmen, Stammdaten, Mitgliedsart und Mitgliedschaftsverlauf ändern sowie Ämter ernennen und abgeben SHALL nur der Vorstand (1. und 2. Vorsitzende, Kassenwart, Schriftwart, Oberbootswart) dürfen; Bootswarte und Abnehmer eines Bootes darf zusätzlich der Oberbootswart bestellen. Alle Mitgliederdaten lesen und die Scheine und Lizenzen jedes Mitglieds ändern darf der Vorstand.

#### Scenario: Aufnahme durch den Vorstand
- **WHEN** Eva als 1. Vorsitzende ein neues Mitglied erfasst
- **THEN** wird es gespeichert

#### Scenario: Aufnahme ohne Amt
- **WHEN** Greta ein Mitglied erfassen will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Bootswarte und Oberbootswart
Bootsstammdaten, Anforderungen und Besatzungsregeln eines Bootes ändern SHALL der Oberbootswart, ein Bootswart dieses Bootes oder der Vorstand dürfen. Sperren, Freigaben, Bestätigungen, Abnahmen und Widerrufe SHALL nach der Spec `aemter` berechtigt werden; sie gelten unverändert auch im Web.

#### Scenario: Fremdes Boot
- **WHEN** Ben als Bootswart der Harald Blauzahn die Filou sperren will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Finanzen und Revision
Forderungen, Einzahlungen, Storno, Saisons, Gebührensätze und Jahresnachweise ändern SHALL nur der Kassenwart oder der Vorstand dürfen. Alle Konten, Forderungen und Außenstände lesen SHALL der Vorstand und der Revisor dürfen. Der Revisor SHALL ausschließlich lesen und das Änderungsprotokoll einsehen können, ohne zu ändern.

#### Scenario: Revisor liest und ändert nicht
- **WHEN** ein Revisor das Konto eines Mitglieds öffnet
- **THEN** sieht er alle Buchungen, aber keine Schaltflächen zum Buchen oder Stornieren
- **AND** ein direkter POST auf die Buchungs-Route wird mit 403 abgelehnt

### Requirement: Keine Anzeige nicht erlaubter Aktionen
Schaltflächen, Links und Formulare für Aktionen, die der Benutzer nicht ausführen darf, SHALL nicht angezeigt werden. Das System SHALL die Berechtigung bei jedem schreibenden Aufruf serverseitig erneut prüfen, unabhängig von der Anzeige.

#### Scenario: Ausgeblendet und gesperrt
- **WHEN** Greta die Bootsseite der Filou öffnet
- **THEN** zeigt sie keine Sperren-Schaltfläche
- **AND** ein manuell gesendeter POST auf die Sperren-Route wird mit 403 abgelehnt

### Requirement: Verhalten bei fehlender Berechtigung
Eine nicht berechtigte lesende Anfrage SHALL mit der deutschen Fehlerseite 403, eine nicht berechtigte schreibende Anfrage mit 403 ohne jede Änderung beantwortet werden. Ohne Anmeldung SHALL zur Anmeldung weitergeleitet werden. Ein Mitglied, das nicht mehr aktiv ist, SHALL keinen Zugriff auf Vereinsdaten mehr haben.

#### Scenario: Ausgeschiedenes Mitglied
- **WHEN** Inès im November 2026 mit einer alten Sitzung eine Seite öffnet
- **THEN** wird sie zur Anmeldung geleitet und die Sitzung beendet

# aenderungsprotokoll Specification

## Purpose

Hält jede schreibende Aktion im Web nachvollziehbar mit Urheber, Zeitpunkt und Gegenstand fest, damit Vorstand und Revisoren Änderungen an der Vereinsakte prüfen können.

## Requirements

### Requirement: Eintrag je Änderung
Jede erfolgreiche schreibende Aktion SHALL einen Protokolleintrag mit Zeitpunkt, handelndem Mitglied, Aktionsart, betroffenem Objekt (Art und Kennung), betroffenem Mitglied (falls vorhanden) und einer kurzen deutschen Beschreibung erhalten. Der Eintrag SHALL in derselben Transaktion wie die Änderung gespeichert werden.

#### Scenario: Eintrag bei Sperre
- **WHEN** Eva die Filou sperrt
- **THEN** enthält das Protokoll einen Eintrag „Boot gesperrt“ mit Eva, der Filou und dem Zeitpunkt

#### Scenario: Fehlgeschlagene Änderung
- **WHEN** eine Änderung wegen eines Konflikts oder fehlender Berechtigung scheitert
- **THEN** entsteht kein Protokolleintrag und es bleibt keine Teiländerung zurück

### Requirement: Keine Geheimnisse und Rohdaten
Das Protokoll SHALL keine Anmeldedaten, Sitzungskennungen oder Client-Geheimnisse enthalten und keine vollständigen Vorher-/Nachher-Kopien personenbezogener Felder; bei Änderungen SHALL es die Namen der geänderten Felder nennen.

#### Scenario: Feldnamen statt Werte
- **WHEN** Anna ihre Telefonnummer ändert
- **THEN** nennt der Eintrag das Feld „Telefon“, nicht die alte oder neue Nummer

### Requirement: Einsicht
Das Protokoll SHALL der Vorstand und der Revisor einsehen, filterbar nach Zeitraum, handelndem Mitglied und Aktionsart und neueste zuerst. Jedes Mitglied SHALL die Einträge sehen, die seine eigenen Daten betreffen. Protokolleinträge SHALL nicht änderbar und nicht löschbar sein.

#### Scenario: Eigene Einträge
- **WHEN** Greta ohne Amt ihre Protokollansicht öffnet
- **THEN** sieht sie nur Einträge, bei denen sie handelnd oder betroffen ist

#### Scenario: Revisor filtert
- **WHEN** ein Revisor das Protokoll nach „Einzahlung“ filtert
- **THEN** sieht er alle Einträge dieser Art, neueste zuerst

# Proposal

## Why

Telefonnummern und E-Mail-Adressen sind unter Vereinsmitgliedern kein Geheimnis: wer ein Boot übergibt, eine Crew sucht oder einen Arbeitstermin abstimmt, braucht sie. Bisher sehen sie nur Amtsträger, alle anderen müssen nachfragen. Wer seine Kontaktdaten nicht teilen möchte, soll das selbst mit einem Sperrvermerk bestimmen können.

## What Changes

- **Kontaktdaten (E-Mail, Telefonnummern) sind vereinsintern für alle aktiven Mitglieder sichtbar** – im Verzeichnis, auf Mitgliedsseiten und in der Ämterübersicht.
- **Sperrvermerk**: Jedes Mitglied kann in „Meine Daten“ selbst ein Merkmal „Kontaktdaten nicht anzeigen“ setzen und wieder entfernen. Bei gesetztem Sperrvermerk sehen andere Mitglieder statt der Angaben den Hinweis „Kontaktdaten gesperrt“; nur das Mitglied selbst und der Vorstand sehen sie weiterhin.
- **Mitgliedsnummer, Mitgliedsart, Segelscheine und Funklizenzen** sind für alle aktiven Mitglieder sichtbar (nötig für die Symbole in der Mitgliederliste aus `add-ui-fuehrung`).
- **Unverändert eingeschränkt**: Anschrift, Segelerlaubnis, Konto, Arbeitsstunden – nur für das Mitglied selbst, den Vorstand und nach `rechte`.
- Das Setzen und Entfernen des Sperrvermerks steht im Änderungsprotokoll.

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `mitgliederverzeichnis`: „Kontaktdaten nur für Ämter“ wird zu „Kontaktdaten vereinsintern sichtbar“ mit Sperrvermerk.
- `mitgliederverwaltung`: Mitgliedsseite zeigt Kontaktdaten allen aktiven Mitgliedern (außer bei Sperrvermerk); Mitglied pflegt den Sperrvermerk selbst.
- `rechte`: Rechtematrix – fremde Kontaktdaten sind lesbar, Sperrvermerk ausgenommen.
- `amtsverwaltung`: Ämterübersicht zeigt die Kontaktdaten der Inhaber nach demselben Maßstab.
- `mitglieder`: neues Merkmal „Sperrvermerk für Kontaktdaten“ am Mitglied.

## Impact

- `crates/vereinsakte-core`: Mitglied erhält `contact_blocked: bool` (Standard `false`, abwärtskompatibel beim Einlesen).
- `crates/vereinsakte-storage`: Migration `0003` – Spalte `contact_blocked`; Lesen/Schreiben.
- `crates/vereinsakte-app`: Sichtbarkeitsregel im Verzeichnis, in den Mitgliedsseiten und der Ämterübersicht; Use Case zum Setzen des Merkmals; Protokollaktion.
- `crates/vereinsakte-web`: Vorlagen für Liste, Mitgliedsseite, Ämter, „Meine Daten“ (Schalter), Hinweis „Kontaktdaten gesperrt“; Tests der alten Regel („Greta sieht keine Kontaktdaten“) werden angepasst.
- Datenschutz: Die Voreinstellung „sichtbar“ ist eine Änderung für Bestandsmitglieder; der Verein informiert sie vor dem Start (Hinweis im README und auf der Startseite einmalig, siehe Design).
- Keine neuen Abhängigkeiten.
- Reihenfolge: **1. von 3**, keine Voraussetzung; `add-ui-fuehrung` und `add-aufgabenboard` folgen danach.

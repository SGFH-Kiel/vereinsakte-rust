# Proposal

## Why

Die Weboberfläche zeigt auf fast jeder Seite alles gleichzeitig: neun gleichrangige Textlinks in der Navigation, Karten ohne Gewichtung, lange Formulare und selten genutzte Verwaltungsaktionen (Sperren, Widerrufen, Abnahme) stehen ungefiltert unter den eigentlichen Inhalten. Nutzer müssen lesen statt erkennen. Funktional ist alles da; es fehlt die Führung: Was ist wichtig, was ist der nächste Schritt, was ist nur für Ausnahmen? Mit Symbolen, einer klaren Typografie-Hierarchie und einem ruhigeren Seitenaufbau wird das ohne neue Fachlogik besser.

## What Changes

- **Navigation neu geordnet**: auf dem Desktop eine seitliche Navigation mit Symbolen und Gruppen (Persönlich, Verein, Verwaltung); auf dem Handy eine untere Tab-Leiste mit den vier häufigsten Bereichen und „Mehr“. Konto und Abmelden wandern in ein Kontomenü. Weiterhin ohne JavaScript bedienbar.
- **Symbolsatz**: ein eigener, eingebetteter SVG-Satz (Navigation, Zustände, Aktionen, leere Zustände), ohne externe Ressourcen; Symbole ergänzen immer Text und tragen nie allein Bedeutung.
- **Typografie und Raster**: feste Schriftstufen (Seitentitel, Abschnitt, Beschriftung, Kennzahl), Überzeile mit Bereich, ruhigere Abstände, tabellarische Ziffern; Beschriftungen und Werte klar unterscheidbar.
- **Seitenaufbau mit Führung**: jede Seite beginnt mit Titel, einer Zustandszeile und genau einer Hauptaktion; weitere Aktionen liegen in einem „Weitere Aktionen“-Menü; selten benötigte oder zerstörende Formulare sind eingeklappt.
- **Startseite als Aufgabenseite**: oben „Was ist zu tun?“ mit Handlungshinweisen, darunter Kennzahlen mit Fortschrittsbalken (Arbeitsstunden) und Zustandssymbolen; Reihenfolge nach Dringlichkeit.
- **Listen kompakter**: Name mit Nebenzeile statt vieler Spalten, Initialen-Kreis bei Personen, Zustandsplaketten mit Symbol, Zeilenaktionen als Symbol-Schaltflächen mit Beschriftung für Hilfstechnik; sinnvolle leere Zustände mit nächstem Schritt.
- **Formulare gegliedert**: Felder in benannten Abschnitten, Pflicht und Hinweis klar abgesetzt, Aktionsleiste am Ende (auf dem Handy am unteren Rand sichtbar).
- Keine Änderung an Rechten, Fachregeln, Adressen oder Datenhaltung. Alle bisherigen Funktionen bleiben erreichbar.

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `oberflaeche`: Navigation (Gruppen, Symbole, Tab-Leiste, Kontomenü), Startseite (Aufgabenhinweise, Fortschritt), Erscheinungsbild und responsive Darstellung angepasst; neue Anforderungen für Symbolsatz, Typografie, Seitenaufbau mit einer Hauptaktion, eingeklappte Seltenaktionen, Leerzustände und gegliederte Formulare.

## Impact

- `crates/vereinsakte-web`: `templates/*` (Basislayout, nahezu alle Seiten und Bausteine), `static/app.css`, neue Symbol-Vorlage `templates/_icons.html`, `src/pages.rs` (Navigationsgruppen, Initialen), kleine View-Erweiterungen für Hinweise auf der Startseite.
- `crates/vereinsakte-web/src/tests.rs`: Navigations- und Layout-Tests angepasst, neue Tests (Symbolsatz vollständig, keine Inline-Styles, Fortschrittselement, eingeklappte Aktionen).
- `tools/screenshots.mjs`, `README.md`: Screenshots und Beschreibung aktualisieren.
- Keine neuen Abhängigkeiten, keine Datenbankänderung, keine Schnittstellenänderung.

# Design

## Context

Die Eignungsregeln liegen im Kern (`skipper_eligibility(member, boat, t)`, Co-Skipper-Eignung je Fahrtgebiet) und prüfen bisher einschließlich Segelerlaubnis für ein einzelnes Mitglied. Die Bootsseite ruft sie nur für das angemeldete Mitglied auf. Kontaktdaten und Sperrvermerk kommen mit `add-kontakt-sichtbarkeit`, Personenlinks/Sortierung/Symbole mit `add-ui-fuehrung`. Siehe proposal.md.

## Goals / Non-Goals

**Goals:** Crew- und Skippersuche direkt am Boot, ohne neue Daten; Datenschutz der Segelerlaubnis-Gründe (Beiträge, Stunden) gewahrt.

**Non-Goals:** keine Verfügbarkeitsabfrage („wer hat am Samstag Zeit“), keine Nachrichtenfunktion, keine Export- oder Seriennachricht-Funktion.

## Decisions

**Qualifikation statt Gesamteignung.** Der Kern erhält eine Variante der Eignungsprüfung ohne Segelerlaubnis (`qualified_as_skipper`, `qualified_as_co_skipper`), von der die bestehende Prüfung selbst nur die Segelerlaubnis ergänzt — kein doppelter Regelcode, die Gesamteignung bleibt `qualified && segelerlaubnis`. Grund: Die Segelerlaubnis hängt an Beitrag und Stunden; ihr Fehlen in einer öffentlichen Liste würde Zahlungsrückstände offenbaren. Alternative: Gesamteignung zeigen — würde das leaken; nur für Vorstand zeigen — Sonderfall ohne klaren Nutzen.

**Ein Durchlauf über aktive Mitglieder.** Die Abfrage lädt aktive Mitglieder mit Abnahmen/Widerrufen/Scheinen (vorhandene Strukturen) und filtert mit der Qualifikationsprüfung; bei Vereinsgröße (hunderte) unkritisch, kein Cache. Ergebnis als `Vec<CrewCandidate{person, contact, licenses, member_type}>`.

**Darstellung.** Bereich „Wer darf dieses Boot fahren?“ mit Reitern/Abschnitten Skipper und Co-Skipper je Fahrtgebiet; Standardzustand: Skipperliste offen, Co-Skipper eingeklappt (`<details>`), weil Skipper der häufigste Fall ist. Tabellen mit sortierbaren Köpfen, Symbole und Personenlinks aus `add-ui-fuehrung`. Bei mehr als 25 Einträgen Seiten wie sonst.

**Rechte.** Lesen alle aktiven Mitglieder, keine neue Capability; keine Schreibaktion.

## Risks / Trade-offs

- [Bootserlaubnisse werden für alle sichtbar] → Wie Segelscheine in `add-kontakt-sichtbarkeit` vereinsintern gedacht; Mitglieder mit Sperrvermerk bleiben in der Liste, nur ohne Kontaktdaten (Qualifikation ist keine Kontaktdatenfrage). Falls unerwünscht, später ein zweiter Schalter.
- [Liste suggeriert „darf fahren“, obwohl Segelerlaubnis fehlt] → Hinweis am Listenkopf, im Test abgesichert.
- [Regeln ändern sich (Anforderungen am Boot)] → Liste wird bei jedem Aufruf berechnet, nie gespeichert.

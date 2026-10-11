# Design

## Context

Bootserlaubnisse ergeben sich aus Abnahmen (`checkouts`: bestanden/nicht bestanden) und Widerrufen je Mitglied und Boot; die Bootsseite nutzt sie bisher nur für das angemeldete Mitglied. Kontaktdaten und Sperrvermerk kommen mit `add-kontakt-sichtbarkeit`, Personenlinks, Sortierung und Symbole mit `add-ui-fuehrung`. Siehe proposal.md.

## Goals / Non-Goals

**Goals:** Crewsuche am Boot mit der einfachsten fachlichen Frage: wer hat die Bootserlaubnis.

**Non-Goals:** keine Eignungsprüfung (Skipper-/Co-Skipper-Anforderungen, Segelerlaubnis, Scheine), keine Verfügbarkeitsabfrage, keine Nachrichtenfunktion.

## Decisions

**Nur Abnahme und Widerruf.** Ein Mitglied steht in der Liste, genau wenn für dieses Boot eine bestandene Abnahme vorliegt und kein später wirksamer Widerruf (Regel „Gültigkeit bis auf Widerruf“ der Spec `bootserlaubnis`, ohne die Ruhensregel für ungültige Meldeadressen). Nur aktive Mitglieder. Dafür gibt es eine eigene schmale Abfrage; die Eignungsprüfung wird weder verändert noch aufgerufen. Das hält die Liste erklärbar („wer abgenommen wurde und nicht widerrufen ist“) und verrät nichts über Beiträge, Stunden oder Adressen.

**Berechnung bei jedem Aufruf.** Aus den Abnahmen und Widerrufen des Boots (bereits geladene Strukturen) und den aktiven Mitgliedern; kein Cache, kein gespeicherter Zustand. Ergebnis als `Vec<BoatPermitHolder{person, contact, licenses, member_type}>`.

**Darstellung.** Abschnitt „Wer darf dieses Boot fahren?“ auf der Bootsseite unterhalb von Zustand und Stammdaten, mit Anzahl in der Überschrift; sortierbare Tabelle, am Handy Karten; Personenlinks, Symbole und Kontaktdarstellung aus den vorangehenden Änderungen. Bei mehr als 25 Einträgen Seiten wie sonst; leerer Zustand: „Noch niemand hat die Bootserlaubnis“.

**Rechte.** Lesen alle aktiven Mitglieder, keine neue Capability, keine Schreibaktion.

## Risks / Trade-offs

- [Bootserlaubnisse werden für alle sichtbar] → Wie Scheine und Kontaktdaten vereinsintern gedacht; Mitglieder mit Sperrvermerk bleiben in der Liste, nur ohne Kontaktdaten. Falls unerwünscht, später ein zweiter Schalter.
- [Liste enthält Mitglieder, die faktisch nicht fahren dürfen (ruhende Erlaubnis, fehlende Segelerlaubnis oder Scheine)] → bewusst so gewünscht; die Überschrift sagt „Bootserlaubnis“, nicht „darf fahren“, falls sich die Beschriftung bei der Umsetzung als missverständlich erweist, wird sie geschärft („Mitglieder mit Bootserlaubnis“).

# Design

## Context

Das Verzeichnis liefert Kontaktdaten bisher nur, wenn der Abfragende ein aktuelles Amt hat (`DirectoryView.show_contact`); die Mitgliedsseite und die Ämterübersicht folgen derselben Regel, Mitgliedsnummer und -art ebenso. Die Regel sitzt in der Anwendungsschicht, damit Daten gar nicht erst in die Seite gelangen (auch nicht in htmx-Teilantworten). Mitglieder pflegen ihre Kontaktdaten selbst über „Meine Daten“ (`ClubService::load/save`, Änderungsprotokoll nur mit Feldnamen). Motivation siehe proposal.md.

## Goals / Non-Goals

**Goals:**
- Eine zentrale Sichtbarkeitsregel für alle Stellen, an denen Kontaktdaten erscheinen.
- Sperrvermerk selbst setzbar, durchgängig wirksam, nachvollziehbar.

**Non-Goals:**
- Kein feingranularer Sperrvermerk je Feld oder je Empfängergruppe.
- Keine Sichtbarkeit nach außen; die Anwendung bleibt anmeldepflichtig.
- Keine Änderung der Sichtbarkeit von Anschrift, Konto, Stunden.

## Decisions

**Zentrale Funktion `contact_visible(viewer, subject)`.** Wahr, wenn `viewer == subject`, der Betrachter Vorstand ist (`Capability` für Mitgliederverwaltung) oder `!subject.contact_blocked`. Verzeichnis, Mitgliedsseite und Ämterübersicht rufen sie auf und liefern bei Falsch ein Feld `contact: Contact::Blocked` statt der Werte; die Vorlagen können die Werte nicht ausgeben, weil sie nie im View-Typ stehen (Typ `Contact::{Visible{email,phones}, Blocked}`). Alternative: Filtern in der Vorlage — fehleranfällig und leakt in Teilantworten.

**Vorstand sieht Gesperrtes.** Der Vorstand braucht Kontaktdaten für Verwaltung, Post und Beitragsfragen; Revisoren, Bootswarte und andere Amtsträger nicht. Das Mitglied sieht in „Meine Daten“, dass der Vorstand weiterhin Zugriff hat (Erklärungstext am Schalter).

**Mitgliedsnummer, Art, Scheine, Funklizenzen für alle sichtbar.** Sie sind nicht sensibel und werden für die Symbole der Mitgliederliste gebraucht; das war bisher die Amtsbedingung, die entfällt.

**Merkmal am Mitglied.** `contact_blocked: bool` mit `#[serde(default)]`; Spalte `contact_blocked INTEGER NOT NULL DEFAULT 0` in einer neuen Migration (nächste freie Nummer). Eigene Änderung ist ein eigener Schalter mit eigenem Use Case, damit „Meine Daten“ ihn auch ohne Änderung der übrigen Felder speichern kann; Rechte: nur das Mitglied selbst (Vorstand nicht, da Willenserklärung des Mitglieds).

**Protokoll.** Aktion `member.contact_block_changed` mit Feld „Sperrvermerk“ (ohne Werte der Kontaktdaten); `ACTIONS`-Sweep-Test wird erweitert.

**Information der Bestandsmitglieder.** Die Voreinstellung „sichtbar“ ändert, wer was sieht. Das Update zeigt allen Mitgliedern auf der Startseite einmalig einen Hinweis mit Link zum Schalter, bis das Mitglied „Meine Daten“ gesehen oder den Hinweis geschlossen hat (Zustand serverseitig: Zeitstempel `contact_notice_seen`). Alternative: stilles Umschalten — datenschutzrechtlich unschön. Alternative: Standard „gesperrt“ — widerspricht der Aussage, dass Kontaktdaten vereinsintern öffentlich sind.

## Risks / Trade-offs

- [Kontaktdaten kommen in Teilantworten oder Exporten vor] → View-Typ trägt Werte nur bei Sichtbarkeit; Tests prüfen Verzeichnis, htmx-Teil, Mitgliedsseite und Ämter auf Abwesenheit der Strings.
- [Bestandsmitglieder überrascht] → Hinweis auf der Startseite und im README; Verein sollte zusätzlich informieren.
- [Vorstand sieht Gesperrtes] → in Erklärungstext benannt; Protokoll zeigt Änderungen, nicht Lesezugriffe (wie bisher).
- [Bestehende Tests erwarten die alte Regel] → gezielt anpassen und um Sperrvermerk-Fälle erweitern.

## Migration Plan

Additiv: neue Spalte mit Standard 0 beim Start. Rückbau durch Zurücksetzen des Commits; die Spalte bleibt ungenutzt.

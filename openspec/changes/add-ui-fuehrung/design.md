# Design

## Context

Die Oberfläche besteht aus askama-Vorlagen (`crates/vereinsakte-web/templates`) und einem Stylesheet (`static/app.css`, 233 Zeilen, Tokens und Komponenten, Hell/Dunkel). Die Navigation ist eine Zeile aus neun Textlinks, auf dem Handy hinter einer Checkbox-Schaltfläche. Seiten bestehen aus gleich gewichteten Karten; Verwaltungsformulare (Boot sperren, Abnahme, Widerruf) stehen offen unter den Inhalten. Randbedingungen: strikte CSP (kein Inline-Skript, kein Inline-Style), keine externen Ressourcen, Bedienung ohne JavaScript, 44-px-Ziele, Kontrast 4,5:1, Druckansicht des Jahresnachweises. Motivation siehe proposal.md.

## Goals / Non-Goals

**Goals:**
- Weniger gleichzeitig sichtbar, klarere Reihenfolge, wiedererkennbare Muster auf allen Seiten.
- Alles bleibt ohne JavaScript nutzbar; htmx nur wie bisher für Suche und Zeilenwechsel.
- Reine Darstellungsänderung: Routen, Rechte, Formularfelder und Fachlogik bleiben.

**Non-Goals:**
- Kein Frontend-Framework, kein Icon-Font, keine Webfonts, kein Build-Schritt.
- Kein Kalender-Widget, keine Diagramme, keine neuen Seiten.
- Kein Umbau der Berechtigungen oder der Navigationsziele.

## Decisions

**Symbole als eingebetteter SVG-Sprite.** `templates/_icons.html` enthält ein verstecktes `<svg>` mit `<symbol id="i-…">`; Vorlagen nutzen ein Makro `icon(name)` → `<svg class="icon" aria-hidden="true"><use href="#i-name"/></svg>`. Eigene schlichte 24×24-Konturzeichen (Strichstärke 2, `currentColor`), damit Farbe, Dunkelmodus und Kontrast über CSS laufen. Alternativen: externe Datei `/static/icons.svg` (zweiter Request, `<use>` mit Fragment funktioniert, aber Cache-/Pfadfehler sichtbar als leere Symbole), Icon-Font (Zugänglichkeit, Fremdcode), Unicode-Zeichen (je Plattform unterschiedlich). Der Sprite ist rund 3 KB je Seite; vertretbar. Ein Test stellt sicher, dass jedes verwendete `#i-…` im Sprite existiert.

**Navigation: Seitenleiste am Desktop, untere Leiste am Handy.** Ab 64 rem eine feste Seitenleiste (Marke, Gruppen, Konto unten); zwischen 48 und 64 rem eine schmale Leiste nur mit Symbolen und Text darunter; unter 48 rem oben nur Marke und Kontoschaltfläche, unten eine Tab-Leiste mit festen vier Zielen (Start, Boote, Reservierungen, Arbeit) plus „Mehr“. Die Tab-Belegung ist für alle gleich, damit sich nichts verschiebt; „Mehr“ öffnet per Checkbox-Muster (wie bisher) ein Blatt mit den übrigen Einträgen, nach Rechten gefiltert. Die Gruppenzuordnung kommt aus `pages.rs::navigation` (Gruppe, Symbol, Ziel, Beschriftung) — eine einzige Quelle für Seitenleiste, Tab-Leiste und „Mehr“. Alternative: oberes Menü behalten und nur Symbole ergänzen — löst die Überfrachtung nicht; eine rollenabhängige Tab-Belegung — springt beim Rollenwechsel.

**Seitenkopf als Baustein.** Ein Makro `page_head(eyebrow, title, subtitle)` mit Platz für Hauptaktion und `<details class="menu-more">` für „Weitere Aktionen“. `<details>` statt Skript-Dropdown: tastaturbedienbar, ohne JavaScript, mit dem Browser-Standard für Zustand; ein Klick außerhalb schließt nicht — akzeptiert, weil das Menü kurz ist.

**Eingeklappte Seltenaktionen mit `<details>`.** Formularblöcke werden `<details class="panel">` mit Titel und Symbol in der `<summary>`; bei Fehlern rendert die Vorlage `open`. Damit bleiben Formulare serverseitig unverändert (Fehler-Rerender zeigt dieselbe Seite).

**Startseite: Hinweisliste aus bestehenden Ansichten.** Die View baut `todo: Vec<Todo{icon, text, href, label}>` aus den schon geladenen Zahlen (Erlaubnis, Stunden, Konto, Bestätigungen, Aufgaben); leer → „Alles erledigt“. Keine neuen Abfragen. Sortierung nach Dringlichkeit festgelegt: Segelerlaubnis, Bestätigungen, Stunden, Konto, Aufgaben.

**Fortschritt mit `<progress>`.** Natives Element mit `value`/`max`, per CSS gestaltet, mit sichtbarem Text daneben; kein Inline-Style nötig (Breite über das Element). Wert wird in der View auf 0–`max` begrenzt, Nachkommastellen als Dezimalpunkt im Attribut, Anzeige deutsch im Text.

**Typografie mit Tokens.** Stufen als CSS-Variablen (`--fs-title`, `--fs-section`, `--fs-label`, `--fs-stat`), `font-variant-numeric: tabular-nums` für Zahlen, `max-width: 70ch` für Fließtext, Beschriftungen klein/gedämpft, Werte normal; Systemschrift bleibt (Kein Webfont: Datenschutz, Ladezeit). Überzeile (Bereichsname) klein in Versalien-Optik über `letter-spacing`, nicht per `text-transform`, damit Hilfstechnik den Text normal liest.

**Initialen-Kreis.** Aus dem Namen (erster Buchstabe Vor- und Nachname) in der View; Farbe über eine von sechs Klassen `av-0…5` aus Mitgliedsnummer mod 6 — kein Inline-Style, deterministisch, Kontrast je Klasse geprüft.

**Filterleiste.** Zweispaltiges Raster `list-layout` (Filterleiste 16 rem + Liste) ab 64 rem; darunter `<details class="filter-panel">` mit `<summary>Filter (n)</summary>` über der Liste, wobei `n` die Zahl gesetzter Filter ist. Dasselbe `<form method=get>` wie bisher (Adresse, htmx-Attribute an der Namenssuche), nur anders platziert; Hauptnavigation links und Filterleiste rechts der Navigation, damit zwei Leisten nie nebeneinander um Platz konkurrieren (Filterleiste sticky unter dem Kopf). Alternative: Filter als Dropdown-Leiste — bei drei Filtern und wachsender Zahl weniger gut erweiterbar.

**Sortierung für alle Tabellen.** Gemeinsamer Baustein: `ListQuery` erhält `sort` und `dir` (`?sort=betrag&dir=desc`); jede Liste deklariert ihre sortierbaren Spalten mit Schlüssel und Vergleich (Wert, Datum, deutscher Textvergleich über Sortierschlüssel ohne Diakritika und Groß-/Kleinschreibung). Makro `th_sort(label, key)` rendert den Kopf als Link mit Pfeilsymbol und `aria-sort`; unbekannte Schlüssel ergeben die Standardreihenfolge. Sortiert wird serverseitig vor dem Seitenwechsel (Paginierung), damit Seiten konsistent bleiben. Am Handy, wo Köpfe ausgeblendet sind, steht ein Auswahlfeld „Sortieren nach“. Alternative: Sortierung im Browser per JavaScript — scheitert an Seitenwechsel und CSP-Minimalismus.

**Namen als Links.** Makro `person(id, name)` rendert `<a class="person" href="/mitglieder/{id}">Name</a>`; die Views liefern Personen als `PersonRef{id: Option<MemberNumber>, name}` statt als String, `id` nur für aktive Mitglieder (Gast, Ausgeschiedene → `None` → Text). So kann keine Vorlage versehentlich einen Namen ohne Verlinkung ausgeben, und ein Test findet alle Stellen. Link-Stil: Textfarbe mit Unterstreichung bei Hover/Fokus, damit Tabellen nicht überfärbt wirken; im Protokolltext (Freitext aus Audit-Beschriftungen) werden Namen aus Kennungen aufgelöst, nicht aus Text geparst.

**Umschalter im Arbeitseinsatz-Formular.** Zwei Radiobuttons `umfang=stunden|zeitraum` stehen als Geschwister vor den Feldgruppen `.only-hours` und `.only-range`; CSS blendet per `#umfang-stunden:checked ~ .only-range` bzw. Gegenstück die irrelevante Gruppe aus (ohne `:has`, ohne JavaScript, funktioniert auch bei Fehler-Rerender, weil `checked` serverseitig aus dem Formular gesetzt wird). Ausgeblendete Felder bleiben im DOM, werden vom Server aber anhand von `umfang` ignoriert, sodass die Fehlermeldung „beide Umfangsformen“ nur bei erzwungenem Aufruf entsteht. Dasselbe Muster dient später für andere Formulare mit alternativen Feldgruppen. Alternative: Felder per htmx nachladen — braucht einen Roundtrip für etwas, das CSS kann.

**Listen.** Tabellen bleiben auf dem Desktop Tabellen (sortierbar, vergleichbar), werden aber auf Hauptspalte + Nebenzeile + Zustand + Aktionen reduziert; Details wandern auf die Detailseite. Am Handy weiterhin Karten; Zeilenaktionen in einem `actions-cell` mit Symbol-Schaltflächen (`aria-label`, `title`). Alternative: alles zu Karten — verliert Vergleichbarkeit am Desktop.

**Formulare.** `_forms.html` erhält Makros für `section(title, intro)` und Pflichtkennzeichnung (`*` plus `aria-required`, Legende „* Pflichtfeld“). Die Aktionsleiste wird `position: sticky; bottom: 0` nur unter 48 rem, mit Hintergrund und Rand; Seitenabstand unten gleicht die Höhe aus, damit nichts verdeckt wird (auch über der Tab-Leiste: Formularseiten blenden die Tab-Leiste aus, `body.has-form`).

**Scheine und Mitgliedsart als Symbole.** Pro Schein-/Lizenzart ein Symbol mit `title` (Tooltip) und `<span class="visually-hidden">` (Text für Hilfstechnik); Tooltips allein genügen weder für Touch noch für Tastatur, daher steht derselbe Text auf der Mitgliedsseite und die Zeile ist per Fokus mit `title` lesbar, am Handy erscheint er als Beschriftung unter dem Symbol-Block. Nur vorhandene Scheine werden gezeigt (keine leeren Platzhalter), damit die Zeile kurz bleibt. Mitgliedsart: ein Symbol je Art (Zuordnung in der View, Text im Tooltip).

**Bootstyp-Symbole.** Zwei Symbole `i-jolle` und `i-dickschiff` (Kielboot), Auswahl aus dem Bootstyp des Boots; Tooltip und versteckter Text aus Typ plus Klasse. Ein Makro `boat_type_icon(type, class)` wird in Bootsliste, Bootsseite, Kalender, Reservierungsdetail und Aufgabenkarten verwendet, damit alles gleich aussieht.

**Crewliste im Reservierungsformular.** Die Crew ist eine Liste von Zeilen statt Textarea. Jede Zeile: Person (Namenslink `person`), optional eigener Zeitraum an Bord, Schaltfläche „Entfernen“. Darunter eine Hinzufügen-Zeile (Mitglied-Auswahl, optional „abweichender Zeitraum“ in einem `<details>`) mit „Hinzufügen“. Der Zustand der Liste steht als wiederholte versteckte Felder im Formular (`crew_member[]`, `crew_from[]`, `crew_to[]`). Ohne JavaScript senden „Hinzufügen“ und „Entfernen“ dasselbe Formular mit `action=crew_add` bzw. `crew_remove:<index>`; der Server baut die Liste um und rendert das Formular erneut, ohne zu speichern (Eingaben bleiben). Eine unsichtbare Standard-Speichern-Schaltfläche steht an erster Stelle des Formulars, damit die Eingabetaste speichert und nicht „Hinzufügen“ auslöst. Mit htmx tauschen dieselben Schaltflächen nur den Listenausschnitt aus (`hx-post`, `hx-target`). Doppelte Personen und der Skipper werden beim Hinzufügen abgelehnt. Das Freitextformat „107; 07.07.2026 10:00; …“ entfällt vollständig. Alternative: Mehrfachauswahl (`<select multiple>`) — schlecht bedienbar am Handy und ohne eigenen Zeitraum je Person.

**Gäste und Mindestbesatzung entfallen.** Eingaben: Gäste haben keinen Portalzugang und werden vom Skipper manuell abgerechnet; die Besatzung ändert sich oft bis zur Abfahrt. Im Kern werden `Reservation.guests` und die Mindestbesatzungsprüfung aus der Gesamtprüfung entfernt; die Gebührenermittlung berücksichtigt nur Skipper, Co-Skipper und Crew. Bestehende Datensätze bleiben lesbar: beim Einlesen werden gespeicherte Gäste (Name, Telefon) als Zeilen an die Bemerkung angehängt, damit nichts verloren geht, danach werden sie nicht mehr geschrieben (Speicherspalten/Tabelle bleiben ungenutzt, keine Migration). `Besatzungsregeln` am Boot bleiben (Co-Skipper-Anforderungen), die Felder Mindestmitglieder/-personen werden nicht mehr ausgewertet und in einer späteren Bereinigung entfernt (Gast-Tarifgruppe, Gastname an Forderungen ebenso). Alternative: Zahl der Gäste als Feld behalten — nicht gewünscht, weil nichts daraus folgt.

**Bemerkung als großes Feld.** `<textarea rows="8">` mit Zeichenzähler-Hinweis (bis 5.000 Zeichen), am Handy volle Breite; Anzeige im Reservierungsdetail mit erhaltenen Zeilenumbrüchen, ohne Markdown und ohne Links (anders als Aufgaben).

**Tour aus der Dauer.** `Reservation::is_tour()` im Kern: Ende-Kalendertag (Ortszeit) > Beginn-Kalendertag. Das gespeicherte Feld `tour` wird nicht mehr gelesen oder geschrieben (Spalte bleibt, kein Migrationsschritt; Export/Import ignorieren sie beim Lesen). Alle Stellen, die `r.tour` nutzen (Gebühren, Anzeige, Filter, Audit), wechseln auf die Methode. Alternative: Mindestdauer in Stunden — zu willkürlich; „Übernachtung“ entspricht der Satzungsdefinition. Offen: Nachtfahrt über Mitternacht gilt damit als Tour; akzeptiert, weil sie Übernachtung an Bord bedeutet.

**Bereich „Kalender“ ersetzt „Reservierungen“.** Neue Route `/kalender` mit `?ansicht=liste|kalender&art=alle|reservierungen|termine&boot=&meine=1&monat=2026-07`. Eine View vereinigt Reservierungen und Arbeitstermine zu `CalendarEntry{kind, start, end, title, subtitle, href, mine, icon}`; Liste und Raster rendern dieselbe Sammlung. Bestehende Detail- und Formularrouten (`/reservierungen/{id}`, `/reservierungen/neu?boot=…`, `/arbeit/termine/{id}`) bleiben; `/reservierungen` und `/arbeit/termine` leiten auf `/kalender` (mit passender Art) um. „Reservieren“ liegt als Hauptaktion auf der Bootsseite und als Zeilenaktion in der Bootsliste; das Formular wählt das Boot vor. Alternative: Reservieren weiterhin im Kalender anbieten — der Nutzer wollte es bewusst beim Boot, weil dort Eignung und Sperre sichtbar sind.

**Liste gruppiert nach Woche und Tag.** Gruppenüberschriften „Diese Woche“, „Nächste Woche“, danach „Woche ab 21. Juli“, darunter Tagesüberschriften; leere Tage entfallen. Mehrtägige Einträge stehen beim Startdatum mit Zeitraum und an späteren Tagen des Fensters als „weiter bis …“. Semantisch `<section>` mit `<h2>` Woche und `<h3>` Tag.

**Kalender serverseitig als HTML-Raster.** Monatsraster (`<table>` mit `<caption>`, Wochentagsköpfen, Tageszellen mit Einträgen als Links) und Wochenansicht als Tagesliste; berechnet aus derselben Sammlung. Mehrtägige Einträge erscheinen an jedem Tag; am Handy immer als Tagesliste der Woche. Kein JavaScript-Kalender, keine Bibliothek. Alternative: Zeitstrahl je Boot — später denkbar.

**Reservierung vs. Arbeitstermin im Aussehen.** Reservierung: Segelboot-Symbol, Boot als Titel, Block mit durchgehender blauer Linkskante (Form „Karte“) und Hintergrund blau getönt. Arbeitstermin: Werkzeug-Symbol, Wort „Arbeitstermin“, gestrichelte gelbe Kante und gelb getönter Hintergrund, Plakette mit freien Plätzen. Eigene Beteiligung: Symbol (Haken/Person) plus Text „dabei“/„angemeldet“. Unterscheidung bleibt in Graustufen durch Symbol, Beschriftung und Kantenstil.

**Rahmen im Stil der Webseite.** Die Webseite hat einen weißen Kopf (Logo links, Titel „Segelgruppe“ fett in `#222`, Untertitel grau), Überschriften in Amaranth und einen marineblauen Fuß mit grauem Text und Symbol-Links. Die Vereinsakte übernimmt das: durchgehender weißer Kopf mit Logo, Titel, Untertitel und dem Zusatz „Vereinsakte“ (kleine Plakette in Blau) und rechts Kontomenü; darunter links die marineblaue Seitenleiste (Desktop) bzw. oben nur Kopf und unten die Tab-Leiste (Handy); unter dem Inhalt der marineblaue Fuß mit „Zur Webseite“, Impressum, Datenschutz (Links auf `https://segelgruppe-kiel.de/…`) und © 2026. Tabellen übernehmen den Stil der Webseite: Kopfzeile Blau `#0077b6` mit weißer Schrift, Zebrastreifen `#f2f2f2`, Hover `#90e0ef`. `theme-color` und Favicon aus dem Logo.

**Eigene Kopien von Logo und Schrift.** `static/img/sgfh-logo.svg` (Vereinslogo, ca. 60 KB; Verwendung durch den Verein selbst), `static/fonts/Amaranth-Regular.woff2/-Bold.woff2` (SIL Open Font License 1.1, Lizenztext im selben Ordner und im README genannt) mit `@font-face` und `font-display: swap`. Die TTF-Dateien der Webseite werden zu WOFF2 komprimiert, ca. 25 KB je Schnitt. Die Dateien liegen im Repository, kein Laden von der Webseite zur Laufzeit (CSP `default-src 'self'` bleibt, Datenschutz: keine Anfrage nach außen). Schrift nur für Überschriften und Markentitel wie auf der Webseite; Fließtext bleibt Systemschrift. Damit ist die offene Frage zu Amaranth entschieden.

**Gesamteindruck als Prüfgegenstand.** Eine Abnahmeliste in den Tasks (Anmeldeseite, Fehlerseiten, leere Zustände, Meldungen, Druck, Formulare, Tabellen, Fokuszustände) und ein Screenshot-Vergleich mit der Webseite (Kopf, Fuß, Farbe, Schrift) vor dem Abschluss; das Skript legt Referenzbilder der Vereinsakte ab, die Sichtprüfung geschieht anhand dieser Bilder.

**Farbthema.** Aus dem Stylesheet der Vereinswebseite: Marine `#023047` (Kopf, Seitenleiste, Überschriften), Blau `#0077b6` (Aktionen, Links; 5,0:1 auf Weiß), Cyan `#00b4d8` und Hellcyan `#90e0ef` (Flächen, Hover, Zierde; nie für Text auf Weiß, 2,4:1), Gelb `#ffab00` (Akzent, Arbeitstermine; Text darauf dunkel `#023047`), Logo-Türkis `#0196b2` für Marke/Zierde. Dunkelmodus: Flächen aus Marine abgeleitet (`#06202d`, `#0b2f42`), Aktionen `#48b6e8` mit dunklem Text, Akzent `#ffc247`. Kontraste werden per Skript in `tools/contrast.mjs` geprüft (Teil von Aufgabe 1.2). Schrift: Überschriften in Amaranth (selbst gehostet, siehe unten), Fließtext Systemschrift.

**Archiv unter Protokoll.** Neue Seite `/protokoll/archiv` mit Reitern je Art (Ämter, Mitglieder, Sperren, Reservierungen, Termine); sie liest über bestehende Repositories mit „nur Vergangenes/Beendetes“. Tagesansichten filtern per Standard auf aktuell; `/aemter/historie` entfällt (Weiterleitung auf das Archiv). Mitgliedsseiten zeigen keine beendeten Ämter mehr. Zugriff wie vollständiges Protokoll (Vorstand, Revisoren). Alternative: Filter „auch Beendete“ in jeder Liste — verwischt den Normalfall, genau das, was vermieden werden soll.

**Normalfall.** Voreinstellungen in Formularen (Heimrevier, Skipper = ich, Datum heute) und Zeitraumstandards in `ListQuery`; Ausnahmefilter hinter „Filter“-`<details>`.

**Aufgaben oben im Menü.** Nur der Menüpunkt kommt hier (Ziel: bestehende Aufgabenliste); das Kanban-Board mit Kommentaren, Bildern und Markdown ist `add-aufgabenboard` und setzt auf Navigation, Seitenkopf und Panels dieser Änderung auf.

**Umsetzungsreihenfolge.** Erst Fundament (Sprite, Tokens, Layout, Navigation), dann Bausteine, dann Seiten bereichsweise — nach jedem Bereich Tests und Screenshots, damit keine große Umstellung am Ende scheitert.

## Risks / Trade-offs

- [Viele Vorlagen betroffen, Tests prüfen Texte] → Texte und Beschriftungen unverändert lassen, wo möglich; Tests gezielt anpassen (Navigation) und die Gesamtprüfung `end_to_end` (alle Seiten × alle Mitglieder) bleibt der Sicherheitsgurt.
- [Sticky-Leisten verdecken Inhalt, besonders mit Bildschirmtastatur] → Sticky nur für die Aktionsleiste, ausreichender Seitenabstand, Screenshots bei 375 × 667; Tab-Leiste auf Formularseiten ausgeblendet.
- [`<details>` öffnet nicht automatisch bei Anker-Sprung] → Fehler-Rerender setzt `open` serverseitig; kein Anker nötig.
- [Symbole ohne Beschriftung irritieren] → Navigation und Plaketten immer mit Text; reine Symbol-Schaltflächen nur in Tabellenzeilen mit `aria-label` und `title`.
- [CSP und SVG `use`] → nur Fragmentverweise `#i-…` auf dasselbe Dokument, keine externen Ziele; per Test und im Browser-Screenshot geprüft.
- [Tour-Regel ändert gespeicherte Bedeutung: bisher gesetzte Tour-Häkchen bei eintägigen Reservierungen entfallen] → vor der Umsetzung Bestandsdaten prüfen (Beispieldaten), Hinweis im README; Gebührenberechnung folgt der neuen Regel nur für künftige Anlage von Forderungen, bestehende Forderungen bleiben unverändert.
- [Archiv blendet Daten aus, die jemand im Alltag sucht] → Seiten, die ein Archivdatum brauchen, verweisen mit Hinweis („Ältere Reservierungen im Archiv“); Suche nach Namen im Archiv.
- [Mehr Seitengewicht] → Sprite unter 4 KB, CSS wächst auf geschätzt 12 KB; kein JavaScript neu.

## Migration Plan

Keine Daten- oder Schnittstellenänderung; Auslieferung mit dem Binary. Rückbau durch Zurücksetzen des Commits. Bestehende Lesezeichen funktionieren (Adressen unverändert).

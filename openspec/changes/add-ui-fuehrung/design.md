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

**Listen.** Tabellen bleiben auf dem Desktop Tabellen (sortierbar, vergleichbar), werden aber auf Hauptspalte + Nebenzeile + Zustand + Aktionen reduziert; Details wandern auf die Detailseite. Am Handy weiterhin Karten; Zeilenaktionen in einem `actions-cell` mit Symbol-Schaltflächen (`aria-label`, `title`). Alternative: alles zu Karten — verliert Vergleichbarkeit am Desktop.

**Formulare.** `_forms.html` erhält Makros für `section(title, intro)` und Pflichtkennzeichnung (`*` plus `aria-required`, Legende „* Pflichtfeld“). Die Aktionsleiste wird `position: sticky; bottom: 0` nur unter 48 rem, mit Hintergrund und Rand; Seitenabstand unten gleicht die Höhe aus, damit nichts verdeckt wird (auch über der Tab-Leiste: Formularseiten blenden die Tab-Leiste aus, `body.has-form`).

**Umsetzungsreihenfolge.** Erst Fundament (Sprite, Tokens, Layout, Navigation), dann Bausteine, dann Seiten bereichsweise — nach jedem Bereich Tests und Screenshots, damit keine große Umstellung am Ende scheitert.

## Risks / Trade-offs

- [Viele Vorlagen betroffen, Tests prüfen Texte] → Texte und Beschriftungen unverändert lassen, wo möglich; Tests gezielt anpassen (Navigation) und die Gesamtprüfung `end_to_end` (alle Seiten × alle Mitglieder) bleibt der Sicherheitsgurt.
- [Sticky-Leisten verdecken Inhalt, besonders mit Bildschirmtastatur] → Sticky nur für die Aktionsleiste, ausreichender Seitenabstand, Screenshots bei 375 × 667; Tab-Leiste auf Formularseiten ausgeblendet.
- [`<details>` öffnet nicht automatisch bei Anker-Sprung] → Fehler-Rerender setzt `open` serverseitig; kein Anker nötig.
- [Symbole ohne Beschriftung irritieren] → Navigation und Plaketten immer mit Text; reine Symbol-Schaltflächen nur in Tabellenzeilen mit `aria-label` und `title`.
- [CSP und SVG `use`] → nur Fragmentverweise `#i-…` auf dasselbe Dokument, keine externen Ziele; per Test und im Browser-Screenshot geprüft.
- [Mehr Seitengewicht] → Sprite unter 4 KB, CSS wächst auf geschätzt 12 KB; kein JavaScript neu.

## Migration Plan

Keine Daten- oder Schnittstellenänderung; Auslieferung mit dem Binary. Rückbau durch Zurücksetzen des Commits. Bestehende Lesezeichen funktionieren (Adressen unverändert).

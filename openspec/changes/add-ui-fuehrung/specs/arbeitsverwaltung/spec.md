## MODIFIED Requirements

### Requirement: Arbeitstermine und Anmeldungen
Der Vorstand und der Oberbootswart SHALL Arbeitstermine mit Datum, Beschreibung und Platzzahl anlegen und ändern können. Jedes Mitglied SHALL die Termine im Bereich „Kalender“ nach der Spec `oberflaeche` gemeinsam mit den Reservierungen, aber erkennbar verschieden, als Liste oder Kalender sehen; Standard SHALL der Zeitraum ab heute sein, vergangene Termine stehen nur im Archiv. Jedes Mitglied SHALL sich zu einem Termin an- und abmelden können; die Anmeldung SHALL keine Stunden gutschreiben, solange kein Einsatz erfasst ist. Ein ausgebuchter Termin SHALL keine weitere Anmeldung annehmen.

#### Scenario: Anmelden
- **WHEN** Anna sich zum Arbeitstermin „Frühjahrsputz“ anmeldet
- **THEN** erscheint sie in der Teilnehmerliste und auf ihrer Startseite

#### Scenario: Ausgebucht
- **WHEN** alle Plätze vergeben sind und ein weiteres Mitglied sich anmelden will
- **THEN** zeigt die Seite „ausgebucht“ und es entsteht keine Anmeldung

#### Scenario: Terminkalender
- **WHEN** Anna den Kalender für den März öffnet
- **THEN** erscheint jeder Arbeitstermin an seinem Tag mit Titel, dem Wort „Arbeitstermin“ und freien Plätzen, und ein Klick führt zum Termin

#### Scenario: Vergangene Termine
- **WHEN** Anna den Kalender ohne weitere Angaben öffnet
- **THEN** erscheinen nur Termine ab heute

### Requirement: Arbeitseinsatz erfassen
Ein Mitglied SHALL für sich einen Arbeitseinsatz mit Datum, Gegenstand (Boot oder Verein), Beschreibung und entweder Stunden oder Zeitraum erfassen können, optional mit Bezug zu einem Arbeitstermin oder einer Aufgabe. Das Formular SHALL einen Umschalter „Stunden / Zeitraum“ bieten, der nur die dafür relevanten Felder zeigt (Stunden bzw. Beginn und Ende); Standard SHALL „Stunden“ sein. Der Umschalter SHALL ohne JavaScript funktionieren, und nur die Angaben der gewählten Umfangsform SHALL gewertet werden. Unplausible Angaben (beide Umfangsformen bei erzwungenem Aufruf, fehlender Gegenstand, Boot ohne Bezug) SHALL nach der Spec `arbeitsstunden` abgelehnt werden.

#### Scenario: Einsatz erfassen
- **WHEN** Clara „Segel der Filou geflickt“, 3 Stunden, Gegenstand Filou erfasst
- **THEN** erscheint der Einsatz unbestätigt in ihrer Stundenliste

#### Scenario: Beide Umfangsformen
- **WHEN** zugleich Stunden und Zeitraum angegeben werden
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

#### Scenario: Umschalter Stunden
- **WHEN** das Formular geöffnet wird
- **THEN** ist „Stunden“ gewählt, das Feld „Stunden“ sichtbar und sind „Beginn“ und „Ende“ nicht sichtbar

#### Scenario: Umschalter Zeitraum
- **WHEN** „Zeitraum“ gewählt wird
- **THEN** erscheinen „Beginn“ und „Ende“, das Feld „Stunden“ verschwindet, und beim Speichern zählen nur Beginn und Ende

#### Scenario: Fehler behält die Wahl
- **WHEN** der Zeitraum mit Ende vor Beginn abgeschickt wird
- **THEN** zeigt das Formular den Fehler am Feld, „Zeitraum“ bleibt gewählt und die Eingaben bleiben erhalten

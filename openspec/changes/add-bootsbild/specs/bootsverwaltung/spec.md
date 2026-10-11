## MODIFIED Requirements

### Requirement: Bootsübersicht und Bootsseite
Alle aktiven Mitglieder SHALL eine Liste der Boote mit Bild, Typ, Freigabe für die laufende Saison und Sperrstatus sowie eine Bootsseite mit Bild, Stammdaten, Bootswarten, Abnehmern, Anforderungen und Besatzungsregeln sehen. Ein Boot ohne Bild SHALL einen Platzhalter mit dem Bootstyp zeigen. Die Bootsseite SHALL zeigen, ob das angemeldete Mitglied das Boot aktuell als Skipper oder Co-Skipper führen darf.

#### Scenario: Filou gesperrt
- **WHEN** die Filou gesperrt ist und ein Mitglied die Bootsliste öffnet
- **THEN** steht neben der Filou „gesperrt“ mit Grund und Zeitraum

#### Scenario: Eigene Eignung
- **WHEN** Anna die Bootsseite der Filou öffnet
- **THEN** zeigt die Seite, ob sie Skipper oder Co-Skipper sein darf, und nennt bei „nein“ die fehlende Anforderung

#### Scenario: Bild in der Liste
- **WHEN** die Filou ein Bild hat und die Trigon keines
- **THEN** zeigt die Liste das Foto bei der Filou und einen Platzhalter mit dem Bootstyp-Symbol bei der Trigon

## ADDED Requirements

### Requirement: Bootsbild pflegen
Wer ein Boot nach dieser Spec bearbeiten darf, SHALL ein Bild des Bootes hochladen, ersetzen und entfernen können, mit optionalem Alternativtext; es SHALL höchstens ein Bild je Boot geben. Andere Mitglieder SHALL weder die Bedienelemente sehen noch das Bild ändern können, ein erzwungener Aufruf SHALL mit 403 abgelehnt werden. Das Ändern des Bildes SHALL im Änderungsprotokoll stehen, ohne Bilddaten.

#### Scenario: Bootswart lädt Bild hoch
- **WHEN** Ben als Bootswart der Harald Blauzahn ein Bild hochlädt
- **THEN** erscheint es in der Bootsliste und auf der Bootsseite

#### Scenario: Ersetzen
- **WHEN** ein neues Bild für ein Boot mit Bild hochgeladen wird
- **THEN** ersetzt es das alte, und es gibt weiterhin nur ein Bild

#### Scenario: Nicht berechtigt
- **WHEN** Greta ohne Amt ein Bild für die Filou hochladen will
- **THEN** sieht sie kein Bildfeld, und ein erzwungener POST wird mit 403 abgelehnt

#### Scenario: Bootswart eines anderen Bootes
- **WHEN** Ben, Bootswart der Harald Blauzahn, ein Bild für die Filou hochladen will
- **THEN** wird dies mit 403 abgelehnt

### Requirement: Bootsbild prüfen und ausliefern
Ein Bootsbild SHALL im Format JPEG, PNG, WebP oder GIF vorliegen und höchstens 8 MB groß sein; der Inhalt SHALL anhand der Dateikennung geprüft werden, andere Formate (auch SVG) SHALL abgelehnt werden. Das Bild SHALL nur angemeldeten aktiven Mitgliedern ausgeliefert werden.

#### Scenario: Falsches Format
- **WHEN** eine SVG-Datei oder eine als „boot.jpg“ benannte Textdatei hochgeladen wird
- **THEN** wird sie abgelehnt mit „Nur JPEG, PNG, WebP oder GIF sind erlaubt“ und das bisherige Bild bleibt

#### Scenario: Zu groß
- **WHEN** ein Bild über 8 MB hochgeladen wird
- **THEN** wird es abgelehnt mit Nennung der Grenze

#### Scenario: Nicht angemeldet
- **WHEN** die Bildadresse ohne Anmeldung aufgerufen wird
- **THEN** wird auf die Anmeldung umgeleitet

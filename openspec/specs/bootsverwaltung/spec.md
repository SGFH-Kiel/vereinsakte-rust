# bootsverwaltung Specification

## Purpose

Macht Boote im Web sichtbar und pflegbar: Stammdaten, Anforderungen und Besatzungsregeln, Sperren und Saisonfreigaben sowie Bootsabnahmen und Widerrufe, jeweils mit den Berechtigungen der Ämter.

## Requirements

### Requirement: Bootsübersicht und Bootsseite
Alle aktiven Mitglieder SHALL eine Liste der Boote mit Typ, Freigabe für die laufende Saison und Sperrstatus sowie eine Bootsseite mit Stammdaten, Bootswarten, Abnehmern, Anforderungen und Besatzungsregeln sehen. Die Bootsseite SHALL zeigen, ob das angemeldete Mitglied das Boot aktuell als Skipper oder Co-Skipper führen darf.

#### Scenario: Filou gesperrt
- **WHEN** die Filou gesperrt ist und ein Mitglied die Bootsliste öffnet
- **THEN** steht neben der Filou „gesperrt“ mit Grund und Zeitraum

#### Scenario: Eigene Eignung
- **WHEN** Anna die Bootsseite der Filou öffnet
- **THEN** zeigt die Seite, ob sie Skipper oder Co-Skipper sein darf, und nennt bei „nein“ die fehlende Anforderung

### Requirement: Stammdaten, Anforderungen und Besatzungsregeln pflegen
Berechtigte SHALL ein Boot anlegen, seine Stammdaten, Anforderungsgruppen (Nachweise) und Besatzungsregeln je Fahrtgebiet ändern können; ein Boot anlegen darf nur der Oberbootswart oder der Vorstand. Eingaben SHALL nach den Regeln der Spec `boote` geprüft werden.

#### Scenario: Boot anlegen
- **WHEN** der Oberbootswart die Jolle „Trigon“ mit Typ Jolle anlegt
- **THEN** erscheint sie in der Bootsliste ohne technische Daten

#### Scenario: Ungültige Besatzungsregel
- **WHEN** eine Besatzungsregel mit Höchst- unter Mindestbesatzung eingegeben wird
- **THEN** zeigt das Formular die Fehlermeldung und speichert nichts

### Requirement: Sperren und Freigeben
Berechtigte nach der Spec `aemter` SHALL ein Boot mit Begründung für einen Zeitraum sperren, eine Sperre aufheben und das Boot für eine Saison freigeben können. Das System SHALL festhalten, wer es wann getan hat, und die Handlungen im Änderungsprotokoll führen.

#### Scenario: Bootswart sperrt
- **WHEN** Ben die Harald Blauzahn mit Begründung „Mast defekt“ sperrt
- **THEN** zeigt die Bootsseite die Sperre mit Ben und Zeitpunkt

#### Scenario: Aufheben ohne offene Sperre
- **WHEN** keine unbefristete Sperre besteht und jemand eine aufheben will
- **THEN** zeigt die Seite „keine offene Sperre vorhanden“

### Requirement: Abnahmen und Widerrufe
Ein Abnehmer oder Bootswart des Bootes sowie der Vorstand SHALL die Abnahme eines Mitglieds für ein Boot erfassen können; das abnehmende Mitglied SHALL festgehalten werden. Einen Widerruf SHALL nur der Vorstand erfassen können, mit Begründung. Die Bootserlaubnis des Mitglieds SHALL danach sofort nach der Spec `bootserlaubnis` neu berechnet erscheinen.

#### Scenario: Abnahme
- **WHEN** der Abnehmer der Filou die Abnahme von Greta erfasst
- **THEN** zeigt Gretas Seite die Bootserlaubnis „Filou“ mit Datum und Abnehmer

#### Scenario: Widerruf
- **WHEN** Eva die Bootserlaubnis von Greta für die Filou widerruft
- **THEN** zeigt Gretas Seite die Erlaubnis als widerrufen mit Begründung

#### Scenario: Widerruf ohne Vorstandsamt
- **WHEN** ein Bootswart einen Widerruf erfassen will
- **THEN** wird dies mit 403 abgelehnt

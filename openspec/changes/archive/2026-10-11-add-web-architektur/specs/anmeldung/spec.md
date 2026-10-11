# Spec Delta

## Purpose

Lässt Vereinsmitglieder sich über den Microsoft-Entra-ID-Mandanten des Vereins (OpenID Connect) anmelden, ordnet die Identität einem Mitglied der Vereinsakte zu und hält die Anmeldung in einer Sitzung.

## ADDED Requirements

### Requirement: Anmeldung über Microsoft Entra ID
Die Anwendung SHALL Mitglieder über Microsoft Entra ID per OpenID Connect (Authorization-Code-Flow mit PKCE) anmelden. Es SHALL nur Konten des konfigurierten Mandanten (Tenant-ID im Token) angenommen werden. Geschützte Seiten SHALL ohne Anmeldung zur Anmeldung umleiten.

#### Scenario: Aufruf ohne Anmeldung
- **WHEN** ein nicht angemeldeter Besucher „Meine Daten“ aufruft
- **THEN** wird er zur Anmeldung beim Anbieter weitergeleitet und nach erfolgreicher Anmeldung zu „Meine Daten“ zurückgeführt

#### Scenario: Fremder Mandant
- **WHEN** sich jemand mit einem Microsoft-Konto aus einem anderen Mandanten anmeldet
- **THEN** wird die Anmeldung abgelehnt, auch wenn seine E-Mail zu einem Mitglied passt

#### Scenario: Manipulierter Rückruf
- **WHEN** der Rückruf des Anbieters einen ungültigen oder fremden `state`-Wert enthält
- **THEN** wird die Anmeldung abgelehnt und keine Sitzung angelegt

### Requirement: Zuordnung zum Mitglied
Bei der ersten Anmeldung SHALL eine Identität dem aktiven Mitglied zugeordnet werden, dessen Anmeldekonto (siehe `mitglieder`) der Adresse aus dem ID-Token (Claim `email`, ersatzweise `preferred_username`) ohne Beachtung der Groß-/Kleinschreibung entspricht. Die Kontakt-E-Mail SHALL dabei nicht berücksichtigt werden. Ohne passendes Anmeldekonto oder ohne aktive Mitgliedschaft SHALL die Anmeldung abgelehnt werden.

#### Scenario: Erste Anmeldung
- **WHEN** sich eine Person aus dem Vereins-Mandanten mit der Adresse `Anna.Petersen@sgfh.example` zum ersten Mal anmeldet
- **THEN** ist sie als Mitglied 101 (Anna Petersen) angemeldet und die Identität ist mit Mitglied 101 verknüpft

#### Scenario: Kontakt-E-Mail meldet nicht an
- **WHEN** das Token die Adresse `anna.petersen@example.org` enthält (Annas Kontakt-E-Mail, nicht ihr Anmeldekonto)
- **THEN** wird die Anmeldung abgelehnt

#### Scenario: Unbekannte oder fehlende E-Mail
- **WHEN** das Token keine Adresse enthält oder die Adresse zu keinem Anmeldekonto passt
- **THEN** wird die Anmeldung mit dem Hinweis abgelehnt, sich an den Vorstand zu wenden

#### Scenario: Ausgeschiedenes Mitglied
- **WHEN** sich ein Mitglied anmeldet, das zum aktuellen Zeitpunkt nicht aktiv ist
- **THEN** wird die Anmeldung abgelehnt

### Requirement: Wiedererkennung über die Objekt-Kennung
Die Zuordnung SHALL gespeichert und bei späteren Anmeldungen über die unveränderliche Objekt-Kennung des Kontos (`oid`) wiedererkannt werden, unabhängig von der dann im Token stehenden Adresse.

#### Scenario: Geänderte E-Mail nach der ersten Anmeldung
- **WHEN** sich Anna nach Änderung der Adresse ihres Microsoft-Kontos erneut anmeldet
- **THEN** wird sie über ihre Objekt-Kennung weiterhin als Mitglied 101 erkannt


### Requirement: Sitzung und Abmeldung
Eine erfolgreiche Anmeldung SHALL eine serverseitige Sitzung mit einem Cookie anlegen, das `HttpOnly`, `SameSite=Lax` und außerhalb des Entwicklungsmodus `Secure` ist. Sitzungen SHALL nach einer konfigurierbaren Zeit ohne Aktivität ablaufen. Die Abmeldung SHALL die Sitzung beenden.

#### Scenario: Abmeldung
- **WHEN** ein angemeldetes Mitglied sich abmeldet und danach „Meine Daten“ aufruft
- **THEN** wird es erneut zur Anmeldung weitergeleitet

### Requirement: Schutz vor Cross-Site-Request-Forgery
Zustandsändernde Anfragen (alles außer GET und HEAD) SHALL nur mit gültigem, an die Sitzung gebundenem CSRF-Token angenommen werden.

#### Scenario: Abmeldung ohne Token
- **WHEN** eine Abmelde-Anfrage ohne gültiges CSRF-Token eintrifft
- **THEN** wird sie abgelehnt und die Sitzung bleibt bestehen

### Requirement: Entwicklungs-Login
Im Entwicklungsmodus SHALL die Anwendung ohne Verbindung zu Microsoft Entra ID und ohne deren Konfiguration laufen. Die Anmeldung SHALL dann durch Auswahl eines Mitglieds aus einer Liste erfolgen, die zu jedem aktiven Mitglied seine aktuellen Ämter zeigt. Außerhalb des Entwicklungsmodus SHALL dieser Weg nicht erreichbar sein.

#### Scenario: Start ohne Entra
- **WHEN** die Anwendung im Entwicklungsmodus ohne Tenant-ID, Client-ID und Client-Geheimnis gestartet wird
- **THEN** startet sie, und „Anmelden“ führt zur Auswahlliste statt zu Microsoft

#### Scenario: Auswahl nach Rolle
- **WHEN** die Auswahlliste geöffnet wird
- **THEN** steht bei Ben „Bootswart“, bei Eva „1. Vorsitzende(r)“ und bei Greta kein Amt

#### Scenario: Produktivmodus
- **WHEN** die Anwendung nicht im Entwicklungsmodus läuft und der Entwicklungs-Login aufgerufen wird
- **THEN** antwortet sie mit „nicht gefunden“

### Requirement: Schneller Benutzerwechsel im Entwicklungsmodus
Im Entwicklungsmodus SHALL jede Seite eine deutlich als Entwicklungsmodus gekennzeichnete Leiste zeigen, über die mit einer Auswahl direkt zu einem anderen Mitglied gewechselt werden kann, ohne sich vorher abzumelden. Nach dem Wechsel SHALL dieselbe Seite aus Sicht des neuen Mitglieds angezeigt werden. Außerhalb des Entwicklungsmodus SHALL weder die Leiste noch der Wechsel existieren.

#### Scenario: Von Greta zu Ben
- **WHEN** im Mitgliederverzeichnis als Greta (ohne Amt) in der Leiste Ben gewählt wird
- **THEN** zeigt das Verzeichnis sofort die Kontaktdaten, die Ben als Bootswart sehen darf

#### Scenario: Produktivmodus ohne Leiste
- **WHEN** eine Seite im Produktivmodus aufgerufen wird
- **THEN** enthält sie keine Entwicklungsleiste, und die Wechsel-Adresse antwortet mit „nicht gefunden“

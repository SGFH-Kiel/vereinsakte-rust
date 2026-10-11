## MODIFIED Requirements

### Requirement: Mitgliedsseite
Zu jedem aktiven Mitglied SHALL es eine Seite mit Name, Mitgliedsnummer, Mitgliedsart, Ämtern, Eintrittsdatum, E-Mail und Telefonnummern geben, die jedes aktive Mitglied sehen darf; bei gesetztem Sperrvermerk SHALL statt E-Mail und Telefonnummern „Kontaktdaten gesperrt“ erscheinen, außer für das Mitglied selbst und den Vorstand. Anschrift, Segelerlaubnis, Konto und Arbeitsstunden SHALL nur auf der Seite des eigenen Mitglieds, für den Vorstand und im Rahmen der Spec `rechte` erscheinen.

#### Scenario: Greta sieht Clara
- **WHEN** Greta die Seite von Clara öffnet
- **THEN** sieht sie Name, Ämter, E-Mail und Telefonnummern, aber keine Anschrift und kein Konto

#### Scenario: Greta sieht gesperrte Kontaktdaten
- **WHEN** Clara den Sperrvermerk gesetzt hat und Greta ihre Seite öffnet
- **THEN** sieht sie „Kontaktdaten gesperrt“ statt E-Mail und Telefonnummern

#### Scenario: Vorstand sieht alles
- **WHEN** Eva die Seite von Clara öffnet
- **THEN** sieht sie Kontaktdaten, Segelerlaubnis, Arbeitsstunden und Konto

### Requirement: Kontaktdaten selbst pflegen
Ein Mitglied SHALL seine Anschrift, Telefonnummern und Kontakt-E-Mail ändern und den Sperrvermerk für seine Kontaktdaten setzen und entfernen können; der Schalter SHALL in „Meine Daten“ mit einer Erklärung stehen, wer die Angaben sonst sieht. Mitgliedsnummer, Mitgliedsart, Mitgliedschaftsverlauf und Anmeldekonto SHALL es nicht ändern können. Die Meldeadresse SHALL geprüft werden. Das Setzen und Entfernen des Sperrvermerks SHALL im Änderungsprotokoll stehen.

#### Scenario: Eigene Änderung
- **WHEN** Anna ihre Straße ändert und speichert
- **THEN** zeigt „Meine Daten“ die neue Straße

#### Scenario: Sperrvermerk setzen
- **WHEN** Anna „Kontaktdaten nicht anzeigen“ aktiviert und speichert
- **THEN** sehen andere Mitglieder bei Anna „Kontaktdaten gesperrt“, und das Protokoll nennt das Feld „Sperrvermerk“, nicht die Kontaktdaten

#### Scenario: Sperrvermerk fremd
- **WHEN** Greta per manuell gesendetem POST den Sperrvermerk von Clara setzt
- **THEN** wird dies mit 403 abgelehnt

#### Scenario: Unzulässiges Feld
- **WHEN** Greta per manuell gesendetem POST ihre Mitgliedsart ändert
- **THEN** bleibt die Mitgliedsart unverändert

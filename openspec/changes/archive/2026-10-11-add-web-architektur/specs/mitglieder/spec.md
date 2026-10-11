## ADDED Requirements

### Requirement: Anmeldekonto
Ein Mitglied SHALL optional die Adresse seines Microsoft-Kontos (Anmeldekonto) tragen. Sie SHALL von der Kontakt-E-Mail getrennt sein, ohne Beachtung der Groß-/Kleinschreibung innerhalb der Vereinsakte eindeutig sein und im JSON als Feld `login_email` erscheinen. Ohne Angabe SHALL das Mitglied kein Anmeldekonto haben.

#### Scenario: Kontakt und Anmeldung getrennt
- **WHEN** Anna die Kontakt-E-Mail `anna.petersen@example.org` und das Anmeldekonto `anna.petersen@sgfh.example` hat
- **THEN** sind beide Adressen unabhängig voneinander gespeichert und im JSON als `email` bzw. `login_email` enthalten

#### Scenario: Doppeltes Anmeldekonto
- **WHEN** zwei Mitglieder das Anmeldekonto `Ben.Hansen@sgfh.example` bzw. `ben.hansen@sgfh.example` haben
- **THEN** meldet die Prüfung der Vereinsakte das doppelt vergebene Anmeldekonto

#### Scenario: Ohne Anmeldekonto
- **WHEN** ein Mitglied ohne Feld `login_email` eingelesen wird
- **THEN** gelingt das Einlesen und das Mitglied hat kein Anmeldekonto

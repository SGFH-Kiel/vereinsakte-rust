# Spec Delta

## Purpose

Bildet die Ämter der Segelgruppe nach Satzung (§10 Vorstand, §11 Revisoren, §12 Bootswarte) und Segelordnung (Abnehmer) mit ihren Amtszeiten ab und legt fest, welches Amt welche Handlungen im Modell vornehmen darf.

## ADDED Requirements

### Requirement: Amt
Ein Amt SHALL einem Mitglied eine Rolle für einen Zeitraum zuordnen (Beginn, optionales Ende). Rollen SHALL sein: 1. Vorsitzender, 2. Vorsitzender, Kassenwart, Schriftwart, Oberbootswart, Revisor, Bootswart eines Bootes und Abnehmer eines Bootes. Ein Mitglied SHALL mehrere Ämter haben können, soweit die Satzung es nicht ausschließt.

#### Scenario: Bootswart
- **WHEN** Ben ab 1. April 2026 Bootswart der „Harald Blauzahn“ ist
- **THEN** ist Ben am 1. Mai 2026 Bootswart dieses Bootes, aber nicht eines anderen Bootes

#### Scenario: Amt beendet
- **WHEN** Annas Amt als Kassenwartin am 30. April 2026 endet
- **THEN** ist sie am 1. Mai 2026 nicht mehr Kassenwartin

### Requirement: Ernennung und Abgabe
Ein Mitglied SHALL zu einem Zeitpunkt in ein Amt ernannt werden können und das Amt zu einem späteren Zeitpunkt abgeben können; die Abgabe SHALL das Ende des Amtes setzen. Beendete Ämter SHALL als Historie erhalten bleiben, und eine erneute Ernennung SHALL ein neues Amt mit eigenem Zeitraum ergeben. Das Ende SHALL nicht vor dem Beginn liegen.

#### Scenario: Bootswart gibt Amt ab
- **WHEN** Ben am 1. April 2026 zum Bootswart der „Trigon“ ernannt wird und das Amt am 15. September 2026 abgibt
- **THEN** ist Ben im Juli 2026 Bootswart der „Trigon“, im Oktober 2026 nicht mehr, und das Amt bleibt mit beiden Daten erfasst

#### Scenario: Erneute Ernennung
- **WHEN** Ben 2028 erneut zum Bootswart der „Trigon“ ernannt wird
- **THEN** gibt es für Ben zwei Bootswart-Ämter der „Trigon“ mit getrennten Zeiträumen, und im Oktober 2026 ist er weiterhin kein Bootswart

#### Scenario: Abgabe eines nicht bestehenden Amtes
- **WHEN** ein Mitglied ein Amt abgeben soll, das es zu diesem Zeitpunkt nicht innehat
- **THEN** wird die Abgabe abgelehnt

#### Scenario: Rechte enden mit dem Amt
- **WHEN** Ben nach Abgabe seines Bootswart-Amtes die „Trigon“ sperren will und kein anderes berechtigendes Amt hat
- **THEN** wird die Handlung als nicht berechtigt abgelehnt

### Requirement: Mehrere Bootswarte je Boot
Ein Boot SHALL zu einem Zeitpunkt beliebig viele Bootswarte haben können; die Bootswarte eines Bootes zu einem Zeitpunkt SHALL abfragbar sein.

#### Scenario: Zwei Bootswarte
- **WHEN** Ben und Clara beide Bootswarte der „Trigon“ sind
- **THEN** liefert die Abfrage für „Trigon“ beide

### Requirement: Vorstand
Vorstand SHALL sein, wer zu einem Zeitpunkt eines der Ämter 1. Vorsitzender, 2. Vorsitzender, Kassenwart, Schriftwart oder Oberbootswart innehat. Vorstand im Sinne des § 26 BGB SHALL 1. Vorsitzender, Kassenwart und Schriftwart sein.

#### Scenario: Vorstand abfragen
- **WHEN** der Vorstand zum 1. Mai 2026 abgefragt wird
- **THEN** werden alle Inhaber der fünf Vorstandsämter mit ihrer Rolle geliefert und die drei § 26-Vertreter als solche gekennzeichnet

### Requirement: Satzungsregeln für Vorstandsämter
Die Prüfung der Vereinsakte SHALL melden, wenn ein Vorstandsamt zu einem Zeitpunkt mit einem nicht stimmberechtigten Mitglied besetzt ist (stimmberechtigt sind aktive, nicht vorläufig aufgenommene ordentliche, studentische und Ehrenmitglieder) oder wenn ein Mitglied zugleich zwei verschiedene Vorstandsämter innehat. Jedes Vorstandsamt SHALL zu einem Zeitpunkt höchstens einmal besetzt sein.

#### Scenario: Außerordentliches Mitglied im Vorstand
- **WHEN** ein außerordentliches Mitglied als Schriftwart eingetragen ist
- **THEN** meldet die Prüfung das nicht stimmberechtigte Vorstandsmitglied

#### Scenario: Ämterhäufung
- **WHEN** Anna zugleich Kassenwartin und Schriftwartin ist
- **THEN** meldet die Prüfung die unzulässige Ämterhäufung

#### Scenario: Bootswart und Vorstand
- **WHEN** Ben zugleich Oberbootswart und Bootswart der „Trigon“ ist
- **THEN** meldet die Prüfung nichts

### Requirement: Recht zum Sperren und Freigeben
Ein Boot sperren, eine Sperre aufheben oder ein Boot für eine Saison freigeben SHALL nur ein Bootswart dieses Bootes, der Oberbootswart oder ein Mitglied des Vorstands dürfen. Sperre und Freigabe SHALL festhalten, wer sie wann vorgenommen hat.

#### Scenario: Bootswart sperrt sein Boot
- **WHEN** Ben als Bootswart der „Harald Blauzahn“ diese wegen eines Ruderschadens sperrt
- **THEN** ist das Boot gesperrt, und Ben ist als Sperrender erfasst

#### Scenario: Fremder Bootswart
- **WHEN** Ben die „Trigon“ sperren will, deren Bootswart er nicht ist und ohne Vorstandsamt
- **THEN** wird die Handlung als nicht berechtigt abgelehnt und das Boot bleibt unverändert

### Requirement: Recht zur Bestätigung von Arbeitsstunden
Einen Arbeitseinsatz bestätigen SHALL dürfen: ein Bootswart, wenn der Einsatz auf einen Arbeitstermin oder eine Aufgabe zu seinem Boot verweist; der Oberbootswart und jedes Vorstandsmitglied für jeden Einsatz. Niemand SHALL seine eigenen Einsätze bestätigen dürfen.

#### Scenario: Bootswart bestätigt
- **WHEN** Ben als Bootswart der „Trigon“ Claras Einsatz bei der Aufgabe „Trigon Antifouling“ bestätigt
- **THEN** ist der Einsatz mit Ben und Zeitpunkt als bestätigt erfasst

#### Scenario: Eigene Stunden
- **WHEN** der Oberbootswart seinen eigenen Einsatz bestätigen will
- **THEN** wird die Handlung als nicht berechtigt abgelehnt

#### Scenario: Einsatz ohne Bootsbezug
- **WHEN** ein Bootswart einen Einsatz ohne Bezug zu seinem Boot bestätigen will und kein Vorstandsamt hat
- **THEN** wird die Handlung als nicht berechtigt abgelehnt

### Requirement: Recht zu Abnahme und Widerruf
Eine Bootsabnahme SHALL nur ein Abnehmer dieses Bootes, ein Bootswart dieses Bootes oder ein Vorstandsmitglied erfassen dürfen; das abnehmende Mitglied SHALL festgehalten werden. Einen Widerruf SHALL nur ein Vorstandsmitglied erfassen dürfen.

#### Scenario: Abnehmer
- **WHEN** David als bestellter Abnehmer der „Filou“ Annas Abnahme als bestanden erfasst
- **THEN** ist die Abnahme mit David als Abnehmer erfasst

#### Scenario: Widerruf durch Bootswart
- **WHEN** ein Bootswart ohne Vorstandsamt eine Bootserlaubnis widerrufen will
- **THEN** wird die Handlung als nicht berechtigt abgelehnt

### Requirement: Prüfung erfasster Handlungen
Die Prüfung der Vereinsakte SHALL alle gespeicherten Sperren, Freigaben, Bestätigungen, Abnahmen und Widerrufe melden, deren Urheber zum Zeitpunkt der Handlung nicht berechtigt war (z. B. nach einem Datenimport).

#### Scenario: Unberechtigte Bestätigung im Bestand
- **WHEN** eine eingelesene Vereinsakte eine Bestätigung durch ein Mitglied ohne passendes Amt enthält
- **THEN** meldet die Prüfung diese Bestätigung

# amtsverwaltung Specification

## Purpose

Macht Ämter und Amtszeiten im Web sichtbar und pflegbar: die aktuelle Besetzung für alle, Ernennung und Abgabe für die berechtigten Ämter, die Historie für den Vorstand.

## Requirements

### Requirement: Ämterübersicht
Alle aktiven Mitglieder SHALL eine Übersicht der aktuell besetzten Ämter mit Inhaber und Beginn sehen, gegliedert in Vorstand, Revisoren und Bootswarte/Abnehmer je Boot; Kontaktdaten SHALL nur nach der Spec `mitgliederverzeichnis` erscheinen.

#### Scenario: Besetzung
- **WHEN** Greta die Ämterübersicht öffnet
- **THEN** sieht sie Eva als 1. Vorsitzende und Ben als Bootswart der Harald Blauzahn, jeweils ohne Kontaktdaten

### Requirement: Ernennen
Berechtigte SHALL ein aktives Mitglied mit Beginn in ein Amt ernennen können: den Vorstand für alle Ämter, den Oberbootswart zusätzlich für Bootswarte und Abnehmer. Die Besetzung SHALL anschließend nach den Regeln der Spec `aemter` geprüft werden; Verstöße (z. B. nicht stimmberechtigtes Mitglied, zwei Vorstandsämter) SHALL als Hinweis erscheinen.

#### Scenario: Bootswart ernennen
- **WHEN** der Oberbootswart Greta zur Bootswartin der Filou ernennt
- **THEN** erscheint das Amt in der Übersicht und Greta darf die Filou sperren

#### Scenario: Nicht berechtigt
- **WHEN** Ben ein Vorstandsamt vergeben will
- **THEN** wird dies mit 403 abgelehnt

#### Scenario: Hinweis bei Doppelbesetzung
- **WHEN** der Vorstand ein Mitglied zusätzlich in ein zweites Vorstandsamt ernennt
- **THEN** wird gespeichert und die Prüfung zeigt den Hinweis zur Doppelbesetzung

### Requirement: Amt abgeben
Berechtigte SHALL ein Amt zu einem Zeitpunkt beenden können, ein Mitglied außerdem sein eigenes Amt abgeben. Ein Ende vor dem Beginn und die Abgabe eines nicht innegehabten Amtes SHALL abgelehnt werden. Beendete Ämter SHALL in der Historie bleiben.

#### Scenario: Abgabe
- **WHEN** Ben sein Amt als Bootswart zum heutigen Tag abgibt
- **THEN** erscheint das Amt in der Historie mit Ende und steht nicht mehr in der aktuellen Übersicht

#### Scenario: Ende vor Beginn
- **WHEN** ein Ende vor dem Beginn des Amtes eingegeben wird
- **THEN** zeigt das Formular „Das Amt kann nicht vor seinem Beginn abgegeben werden“

### Requirement: Amtshistorie
Der Vorstand SHALL die Amtshistorie mit allen beendeten und laufenden Ämtern je Mitglied und je Amt einsehen können.

#### Scenario: Historie eines Amtes
- **WHEN** der Vorstand die Historie des Kassenwarts öffnet
- **THEN** sieht er alle bisherigen Inhaber mit Beginn und Ende

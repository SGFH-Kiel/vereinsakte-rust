## MODIFIED Requirements

### Requirement: Amt abgeben
Berechtigte SHALL ein Amt zu einem Zeitpunkt beenden können, ein Mitglied außerdem sein eigenes Amt abgeben. Ein Ende vor dem Beginn und die Abgabe eines nicht innegehabten Amtes SHALL abgelehnt werden. Beendete Ämter SHALL nur noch im Archiv erscheinen und in keiner Tagesansicht, auch nicht in der Ämterübersicht oder auf Mitgliedsseiten.

#### Scenario: Abgabe
- **WHEN** Ben sein Amt als Bootswart zum heutigen Tag abgibt
- **THEN** erscheint das Amt im Archiv mit Ende und steht nicht mehr in der Ämterübersicht und nicht mehr bei Ben

#### Scenario: Ende vor Beginn
- **WHEN** ein Ende vor dem Beginn des Amtes eingegeben wird
- **THEN** zeigt das Formular „Das Amt kann nicht vor seinem Beginn abgegeben werden“

### Requirement: Amtshistorie
Der Vorstand und die Revisoren SHALL die Amtshistorie mit allen beendeten und laufenden Ämtern je Mitglied und je Amt im Archiv einsehen können; es gibt keine eigene Historienseite im Bereich Ämter.

#### Scenario: Historie eines Amtes
- **WHEN** der Vorstand im Archiv die Ämter des Kassenwarts öffnet
- **THEN** sieht er alle bisherigen Inhaber mit Beginn und Ende

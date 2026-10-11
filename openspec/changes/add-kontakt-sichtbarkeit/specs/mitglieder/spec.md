## ADDED Requirements

### Requirement: Sperrvermerk für Kontaktdaten
Ein Mitglied SHALL ein Merkmal „Sperrvermerk für Kontaktdaten“ tragen können. Ohne Angabe SHALL es nicht gesetzt sein, und beim Einlesen von Daten ohne dieses Merkmal SHALL es als nicht gesetzt gelten.

#### Scenario: Voreinstellung
- **WHEN** ein Mitglied neu aufgenommen wird
- **THEN** ist der Sperrvermerk nicht gesetzt

#### Scenario: Alte Daten
- **WHEN** eine Akte ohne das Merkmal eingelesen wird
- **THEN** gilt der Sperrvermerk bei allen Mitgliedern als nicht gesetzt

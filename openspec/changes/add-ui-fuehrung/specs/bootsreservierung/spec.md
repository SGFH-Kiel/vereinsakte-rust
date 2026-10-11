## MODIFIED Requirements

### Requirement: Tour-Kennzeichen
Eine Reservierung SHALL als Tour gelten, wenn ihr Zeitraum mindestens eine Übernachtung umfasst, das heißt ihr Ende auf einem späteren Kalendertag liegt als ihr Beginn. Die Eigenschaft SHALL sich allein aus dem Zeitraum ergeben und nicht angegeben werden können; ändert sich der Zeitraum, ändert sich die Eigenschaft mit.

#### Scenario: Kurze Ausfahrt
- **WHEN** ein Boot für einen Nachmittag reserviert wird
- **THEN** ist die Reservierung keine Tour

#### Scenario: Mehrtägige Fahrt
- **WHEN** ein Boot vom 1. bis 7. Juli reserviert wird
- **THEN** gilt die Reservierung als Tour

#### Scenario: Zeitraum verkürzt
- **WHEN** eine Tour vom 1. bis 7. Juli auf den 1. Juli nachmittags verkürzt wird
- **THEN** gilt sie nicht mehr als Tour

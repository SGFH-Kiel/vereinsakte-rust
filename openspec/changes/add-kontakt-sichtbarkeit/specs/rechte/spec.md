## MODIFIED Requirements

### Requirement: Rechtematrix
Die Berechtigung SHALL je Bereich nach der Rechtematrix dieser Spec bestimmt werden. Alle aktiven Mitglieder lesen Verzeichnis (einschließlich E-Mail und Telefonnummern, soweit kein Sperrvermerk gesetzt ist), Boote (ohne interne Vermerke), Arbeitstermine, Aufgaben, Ämter und Reservierungskalender. Fremde Konten, Forderungen, Anschriften und Arbeitsstunden SHALL nur lesen, wer in den folgenden Anforderungen dafür genannt ist.

#### Scenario: Mitglied ohne Amt
- **WHEN** Greta ohne Amt das Mitgliederverzeichnis, die Boote und den Reservierungskalender öffnet
- **THEN** sieht sie alle drei einschließlich der nicht gesperrten Kontaktdaten, aber weder fremde Konten noch fremde Anschriften

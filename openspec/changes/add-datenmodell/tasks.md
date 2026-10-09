# Tasks

Konvention: Bezeichner und Code englisch (Glossar in design.md), Doku-Kommentare und `Display`-Texte deutsch.

Testdaten: Alle Tests bauen auf einem gemeinsamen Testmodul `example` mit realistischen, erfundenen Vereinsdaten auf (Mitglieder mit plausiblen Namen, Kieler Adressen und Mitgliedsnummern, die echten Bootsnamen, Termine der Saison 2026). Jede Gruppe erweitert `example` um die Daten, die ihre Tests brauchen, statt in jedem Test eigene Minimaldaten anzulegen. Randfälle (Jahreswechsel, Crewwechsel, Überzahlung …) werden als benannte Fälle in `example` gepflegt, damit sie wiederverwendbar und lesbar bleiben.

## 1. Grundgerüst und Abhängigkeiten

- [ ] 1.1 In `Cargo.toml` `serde` (Feature `derive`), `chrono` (Feature `serde`), `uuid` (Features `v4`, `serde`), `rust_decimal` und als dev-dependencies `serde_json` und `rust_decimal_macros` eintragen; verifizieren mit `cargo build`
- [ ] 1.2 `src/model/mod.rs` mit leeren Untermodulen `ids`, `time`, `money`, `member`, `office`, `boat`, `requirement`, `boat_permit`, `work_hours`, `sailing_permit`, `reservation`, `work_event`, `task`, `account`, `fees`, `tour_fees`, `club_record`, `issues` und `#[cfg(test)] example` anlegen und in `src/main.rs` per `#[allow(dead_code)] mod model;` einbinden (bis die Anwendung das Modell nutzt, sonst scheitert Clippy an ungenutztem Code); verifizieren mit `cargo build`

## 2. Datenformat: Kennungen, Zeitpunkte, Geld, ClubRecord

- [ ] 2.1 In `ids` `MemberId(NonZeroU32)` (serde über `u32`) und per Makro die UUID-Newtypes `BoatId`, `OfficeId`, `CheckoutId`, `RevocationId`, `WorkEntryId`, `HourTransferId`, `ReservationId`, `WorkEventId`, `TaskId`, `ClaimId`, `PaymentId` erzeugen; Tests: `MemberId` 42 → JSON `42`, `0`, `-5` und `"42"` werden abgelehnt, zwei `BoatId::new()` verschieden, UUID-JSON ist Zeichenkette, ungültige UUID schlägt fehl (`cargo test`)
- [ ] 2.2 In `time` `Timestamp = DateTime<FixedOffset>` definieren; Tests: `2026-05-01T10:00:00+02:00` übersteht den JSON-Rückweg unverändert, `"2026-05-01T10:00:00"` und `"2026-05-01"` werden abgelehnt (`cargo test`)
- [ ] 2.3 In `money` `Money(Decimal)` mit Normalisierung auf 2 Nachkommastellen, `from_euros`, `from_cents`, `ZERO`, `FromStr`, `Add`, `Sub`, `Neg`, `Sum`, `Mul<Decimal>`, `prorate`, deutschem `Display` und eigener serde-Implementierung als String umsetzen; Tests: 75 € → JSON `"75.00"` und „75,00 €“; `"240"` → 240,00 € → `"240.00"`; `75.0` als Zahl und `"75.005"` werden abgelehnt; 15 € × 2,5 = 37,50 €; 240 € × 6/12 = 120,00 €; 170 € × 5/12 = 70,83 €; Summe und negativer Betrag „-3,50 €“ (`cargo test`)
- [ ] 2.4 `ClubRecord` mit allen Listen (in späteren Gruppen um die jeweiligen Typen ergänzt) und `serde(default, skip_serializing_if)` anlegen; Test: `{}` liest sich als leere Akte, leere Akte wird als `{}` geschrieben (`cargo test`)
- [ ] 2.5 Testmodul `example` mit Grundgerüst anlegen (Hilfsfunktionen für Zeitpunkte in Vereinszeit, z. B. `at(2026, 5, 1, 10, 0)`, und eine erste `ClubRecord`); Test: leere Beispielakte lässt sich erzeugen (`cargo test`)

## 3. Mitglieder

- [ ] 3.1 `SailingLicenseType` und `RadioLicenseType` mit `#[serde(into = "String", from = "String")]`, `From`-Implementierungen und `Display` umsetzen; Tests: `Sks` → `"SKS"`, `SbfSee` → `"SBF See"`, `"sbf see"` → `SbfSee`, `" SRC "` → `Src`, `"Jüngstenschein"` → `Other` und zurück unverändert, `Other("SKS")` liest sich als `Sks` zurück (`cargo test`)
- [ ] 3.2 `Address`, `PhoneNumbers`, `SailingLicense`, `RadioLicense`, `MembershipType` (mit `pays_fee`, `owes_hours`, `tariff_group`, `has_voting_right`), `Membership` und `Member` (Kennung `id: MemberId`, `email`, `address_valid` default `true`, `child_of`, `board_consent`) implementieren; Tests: Mitgliedsnummer 42 als `"id": 42` und PLZ „01067“ bleiben erhalten, fehlendes `last_name` schlägt fehl, fehlendes `landline` wird nicht geschrieben, Pflichten-, Tarif- und Stimmrechtstabelle je Mitgliedsart, fehlendes `address_valid` ergibt `true` (`cargo test`)
- [ ] 3.3 `Member::is_active_at`, `is_active_in`, `is_provisional_at`, `declare_resignation`, `has_sailing_license`, `has_radio_license` implementieren; Tests: Austritt am 10.05.2026 → aktiv im Dezember 2026, nicht am 01.01.2027; Erasmus befristet bis Ende Oktober → im November nicht aktiv; Ausschluss 01.06. → am 02.06. nicht aktiv; vorläufig erkennbar; SBF See + SKS → SKS ja, SSS nein (`cargo test`)
- [ ] 3.4 `RecordIssue` und `ClubRecord::validate()` mit Prüfung auf doppelte Mitgliedsnummern anlegen; Tests: zwei Mitglieder 42 → Meldung nennt 42; eindeutige Nummern → `Ok` (`cargo test`)

## 4. Boote und Anforderungen

- [ ] 4.1 `BoatType`, `SailingArea`, `Qualification`, `AnyOf`, `Requirement`, `CrewRule`, `Release`, `Block` und `Boat` gemäß design.md implementieren; Tests: „Trigon“ mit Klasse/Baujahr/Länge und „Filou“ nur mit Name/Typ überstehen den JSON-Rückweg; Qualification-JSON hat die Form `{"sailing_license": "SKS"}` bzw. `{"boat_permit_for_type": "dinghy"}` (`cargo test`)
- [ ] 4.2 `Boat::crew_rule(area)` und `Boat::availability_at(t)` (Saisonfreigabe, Sperren mit exklusivem Ende) implementieren; Tests: nur 2025 freigegeben → Mai 2026 nicht freigegeben; Sperre ab 03.06. ohne Ende → 10.06. gesperrt mit Grund „Ruderschaden“; Sperre bis 15.06. → 16.06. nutzbar; Jolle ohne Regel „outside“ → `None` (`cargo test`)

## 5. Ämter

- [ ] 5.1 `Role` (mit `is_board`, `is_legal_representative`) und `Office` implementieren und in `ClubRecord` aufnehmen; `holders(role, t)`, `boatwardens(boat, t)`, `board(t)`; Tests: Role-JSON `"chair"` bzw. `{"boatwarden": "<uuid>"}`; Ben Bootswart der „Harald Blauzahn“ ab 01.04. → am 01.05. ja, für „Trigon“ nein; Amt bis 30.04. → am 01.05. nicht mehr; zwei Bootswarte der „Trigon“ → beide geliefert; Vorstand mit § 26-Kennzeichnung (`cargo test`)
- [ ] 5.2 `ClubRecord::appoint(member, role, start)` und `resign(member, role, at)` mit `OfficeError` implementieren; Tests: Ben ernannt 01.04.2026, Abgabe 15.09.2026 → im Juli Bootswart, im Oktober nicht, Amt bleibt mit beiden Daten erhalten; erneute Ernennung 2028 → zwei getrennte Ämter; Abgabe ohne aktives Amt bzw. vor Beginn → Fehler (`cargo test`)
- [ ] 5.3 Satzungsprüfung der Vorstandsämter in `validate()` ergänzen; Tests: außerordentliches oder vorläufiges Mitglied als Schriftwart → `BoardMemberWithoutVote`; Kassenwart + Schriftwart gleichzeitig → `BoardOfficesAccumulated`; zwei Kassenwarte gleichzeitig → `BoardOfficeTwice`; Oberbootswart + Bootswart → keine Meldung (`cargo test`)

## 6. Bootserlaubnis

- [ ] 6.1 `CheckoutResult`, `BoatCheckout` (mit Pflichtfeld `examiner`), `Revocation` (mit `by`) und `PermitMissing` implementieren und in `ClubRecord` aufnehmen; Test: Abnahmen und Widerruf überstehen den JSON-Rückweg, fehlender `examiner` schlägt fehl (`cargo test`)
- [ ] 6.2 `ClubRecord::boat_permit(member, boat, t)` implementieren; Tests: bestanden 2019 → gültig 2026 für dieses Boot, nicht für ein anderes; Widerruf danach → `Revoked`; neue bestandene Abnahme nach Widerruf → wieder gültig; nur nicht bestanden → `NoCheckout`; ungültige Meldeadresse → `AddressInvalid`, nach Korrektur wieder gültig (`cargo test`)
- [ ] 6.3 `ClubRecord::check_requirement(member, &Requirement, t)` implementieren (inkl. `BoatPermit(BoatId)` und `BoatPermitForType`); Tests: leere Anforderung erfüllt; [SKS|SSS|SHS|BR] mit BR erfüllt; „Segelkenntnisse“ nur mit Jollen-Bootserlaubnis erfüllt; Dehler-Anforderung ohne Funklizenz → genau die Gruppe [SRC|LRC] fehlt (`cargo test`)

## 7. Arbeitsstunden

- [ ] 7.1 `WorkExtent`, `WorkSubject`, `WorkReference`, `Confirmation`, `RawWorkEntry` und `WorkEntry` mit `serde(try_from/into)` sowie `hours()`, `year()`, `validate()` implementieren und in `ClubRecord` aufnehmen; Tests: (12.04.2026, 3,5 h, Beschreibung) zählt 3,5; 09:00–13:30 zählt 4,5; JSON mit `hours` und `start`/`end` sowie JSON ohne Umfang wird abgelehnt; −2 h und Ende vor Start ungültig; Start 31.12.2026 22:00 +01:00 → Jahr 2026; JSON ist flach; neuer Eintrag ist unbestätigt; Eintrag ohne Termin/Aufgabe mit Gegenstand „Filou“ gültig; Gegenstand `"club"` gültig; fehlender Gegenstand schlägt beim Einlesen fehl (`cargo test`)
- [ ] 7.2 `HourTransfer` mit `validate()` implementieren und in `ClubRecord` aufnehmen; Tests: Übertrag an sich selbst und mit 0 bzw. negativen Stunden ungültig (`cargo test`)

## 8. Arbeitstermine und Aufgaben

- [ ] 8.1 `WorkEvent`, `EventError`, `RegistrationError`, `validate()`, `register()`, `unregister()` implementieren und in `ClubRecord` aufnehmen; Tests: „Boote kranen“ übersteht den JSON-Rückweg; Ende nicht nach Beginn ungültig; doppelte Anmeldung abgelehnt und Liste unverändert; dritter Teilnehmer bei max. 2 abgelehnt; Abmeldung entfernt (`cargo test`)
- [ ] 8.2 `TaskStatus`, `Task`, `assign()`, `unassign()`, `set_status()` und `ClubRecord::hours_for_task()` implementieren und in `ClubRecord` aufnehmen; Tests: neue Aufgabe ist offen; doppelte Zuweisung ändert nichts; „done“ setzt `done_at`, Zurücksetzen löscht es; Einträge 2,5 h + 1,5 h mit Bezug auf die Aufgabe → 4 h (`cargo test`)
- [ ] 8.3 `ClubRecord::validate_work_entry(&entry)` implementieren und in `validate()` aufnehmen; Tests: Einsatz mit Verweis auf Aufgabe „Trigon Antifouling“ und Gegenstand „Filou“ → Widerspruch gemeldet; gleicher Verweis mit Gegenstand „Trigon“ → `Ok`; Verweis auf Termin ohne Boot mit beliebigem Gegenstand → `Ok` (`cargo test`)

## 9. Rechte

- [ ] 9.1 `Action`, `NotAuthorized` und `ClubRecord::authorize(actor, &Action, t)` gemäß Tabelle in design.md implementieren; Tests: Bootswart sperrt eigenes Boot → `Ok`, fremdes Boot → abgelehnt; Oberbootswart und Kassenwart dürfen jedes Boot sperren/freigeben; Bootswart bestätigt Einsatz mit Gegenstand seines Bootes → `Ok`, mit fremdem Boot → abgelehnt; Einsatz mit Gegenstand „Verein“ darf jeder aktive Amtsinhaber (auch Revisor, Abnehmer) bestätigen, ein Mitglied ohne Amt nicht; niemand bestätigt eigene Einsätze; Abnehmer und Bootswart erfassen Abnahme ihres Bootes, Widerruf nur Vorstand; nach `resign` berechtigt das Amt nicht mehr (`cargo test`)
- [ ] 9.2 Mutierende Methoden `block_boat`, `lift_block`, `release_boat`, `confirm_work`, `record_checkout`, `revoke` implementieren, die vorher `authorize` aufrufen und Urheber/Zeitpunkt speichern; Tests: erfolgreiche Sperre speichert `by`; abgelehnte Handlung lässt `ClubRecord` unverändert (Vergleich vorher/nachher); `confirm_work` setzt `Confirmation` (`cargo test`)
- [ ] 9.3 `validate()` um die Prüfung gespeicherter Handlungen erweitern (`RecordIssue::Unauthorized`); Test: per JSON eingelesene Bestätigung durch Mitglied ohne Amt wird gemeldet, berechtigte nicht (`cargo test`)

## 10. Mitgliedskonto

- [ ] 10.1 `ClaimKind`, `Cancellation`, `Claim`, `Payment` und `MemberAccount` (als Feld `account` im `Member`) mit `validate()`, `cancel()` und `has_claim()` implementieren; Tests: Mitglied ohne `account` im JSON hat leeres Konto und wird ohne `account` geschrieben; ClaimKind-JSON `{"annual_fee": {"year": 2026}}`, `"admission_fee"`; Betrag 0 oder negativ ungültig; stornierte Forderung bleibt sichtbar; `has_claim` findet auch stornierte (`cargo test`)
- [ ] 10.2 `MemberAccount::total_claimed`, `total_paid`, `balance` und `claim_status` (Tilgung nach Fälligkeit) implementieren; Tests: Forderungen 240 € + 36 €, Einzahlung 240 € → Kontostand 36 €; 240 € (April) + 36 € (Juli), Einzahlung 250 € → Beitrag bezahlt, 26 € der Tourgebühr offen; stornierte Forderung zählt nicht; Überzahlung → alles bezahlt, negativer Kontostand; Σ offen = max(Kontostand, 0) (`cargo test`)
- [ ] 10.3 `ClubRecord::add_claims` und `outstanding(t)` implementieren; Tests: Tourgebühr fällig 08.07., offen am 20.07. → überfällig; nicht fällige offene Beträge zählen als offen, nicht als überfällig; alles bezahlt → leere Übersicht (`cargo test`)

## 11. Saison, Beiträge und Segelerlaubnis

- [ ] 11.1 `Season`, `FeeSchedule` (Default nach Beitragsordnung), `TypeRate`, `TourRate`, `Reduction`, `Compensation`, `AnnualRecord` implementieren und in `ClubRecord` aufnehmen; `hours_quota(year)`, `compensation_rate(year)`, `fees(year)`, `compensation_amount(...)`; Tests: ohne Saison 2027 → 25 h / 15 € / Standardtabelle; Saison 2026 mit 30 h → 30 h; Saison 2027 mit ordentlichem Beitrag 260 € → 260 €; 5 h Abgeltung ohne Betrag → 75,00 € (`cargo test`)
- [ ] 11.2 `annual_fee_due_date(year)` mit Osterberechnung, `ClubRecord::annual_fee_claims(year)` und `admission_fee_claims()` implementieren; Tests: ordentlich seit 2020 → 240 €; Eintritt Juli 2026 → 120 €; Ehrenmitglied und Kind → keine Forderung; Erasmus Eintritt März → 100 € nicht anteilig, keine Aufnahmegebühr; studentisch → Aufnahme 30 €; 1. April 2023 (Samstag) → 3. April; 2024 (Ostermontag) → 2. April; Eintritt im Juni → fällig mit Eintritt; zweiter Lauf erzeugt keine weiteren Forderungen (`cargo test`)
- [ ] 11.3 `ClubRecord::compensation_claims()`, `work_entries_of(member, year)` und `credited_hours(member, year)` implementieren; Tests: Abgeltung 75 € → Forderung `WorkCompensation { year: 2026 }` über 75,00 € im Konto, zweiter Lauf ohne Doppelung; bestätigte Einträge 4 + 2,5 + 8 h, unbestätigt 6 h, +5 h erhalten, −2 h abgegeben, +3 h abgegolten → 20,5; `work_entries_of` liefert auch den unbestätigten; nur Einträge 2025 → 2026 = 0; bloße Anmeldung zu einem Termin ändert nichts (`cargo test`)
- [ ] 11.4 `SailingPermitMissing` (mit deutschem `Display`), `ClubRecord::fee_arrears`, `required_hours`, `sailing_permit(member, year)` und `sailing_permit_at(member, t)` implementieren; Tests: Beitrag gefordert und bezahlt + 25/25 h → `Ok`; 20/25 h → `HoursMissing { missing: 5.0 }`; Beitrag nicht gefordert → `FeeArrears`; Einzahlung tilgt erst ältere Tourgebühr → `FeeArrears(36 €)`; Rückstand + Stunden fehlen → beide Gründe; Ehrenmitglied ohne Forderung → kein Rückstand; Kind → Soll 0; reduziertes Soll 10 h wirkt; außerordentlich ohne Zustimmung → `NoBoardConsent`; 2026 nie aktiv → `Inactive`; im März ausgetreten → `sailing_permit_at` im Juni schlägt fehl (`cargo test`)

## 12. Eignung als Skipper und Co-Skipper

- [ ] 12.1 Testmodul `example` um die fünf Vereinsboote mit ihren Regeln gemäß Tabelle in design.md (Filou und Dipl.Ing.ling als Jollen), Bootswarte, Vorstand sowie Testmitglieder mit gefordertem und bezahltem Beitrag und bestätigten Stunden vervollständigen; Test: jedes Boot liefert für „home_waters“ eine Regel, nur Folkeboot/Dehler/Matcher für „outside“, `validate()` ist `Ok` (`cargo test`)
- [ ] 12.2 `EligibilityIssue` und `ClubRecord::skipper_eligibility(member, boat, t)` implementieren; Tests mit `example`: Segelerlaubnis + Bootserlaubnis „Schruvtrekker“ + SKS, SBF See, SRC → geeignet; ohne Bootserlaubnis und ohne Funklizenz → beide Gründe; Jolle „Dipl.Ing.ling“ nur mit Bootserlaubnis → geeignet (`cargo test`)
- [ ] 12.3 `ClubRecord::co_skipper_eligibility(member, boat, area, t)` implementieren; Tests: Dehler „outside“ mit Bootserlaubnis „Harald Blauzahn“ + SBF See, ohne Dehler-Erlaubnis → geeignet; nur SBF See → Gruppe Bootserlaubnis fehlt; Fahrtgebiet ohne Regel → `AreaNotReleased` (`cargo test`)

## 13. Bootsreservierung

- [ ] 13.1 `CrewSeat`, `Guest` (mit `invited_by`, `from`, `until`), `Reservation` (mit `tour`), `ReservationError`, `validate()`, `period_of()` und `overlaps()` implementieren und in `ClubRecord` aufnehmen; Tests: Reservierung mit Co-Skipper, Crew und Gast übersteht den JSON-Rückweg; `tour` fehlt → `false`; Ende vor/gleich Beginn, Skipper = Co-Skipper, Skipper/Co-Skipper in Crew, doppeltes Crew-Mitglied, Crew-Zeitraum außerhalb der Reservierung, mehrere Verstöße zugleich; 10–14/14–18 keine, 10–14/13–18 eine Überschneidung, verschiedene Boote keine (`cargo test`)
- [ ] 13.2 `ReservationIssue` (mit deutschem `Display`) und `ClubRecord::check_reservation(&r)` implementieren; Tests mit `example`: zulässige Jollenreservierung allein → `Ok`; Crew-Mitglied ohne Segelerlaubnis → beanstandet mit Mitglied; Skipper ohne Bootserlaubnis → beanstandet; gesperrtes Boot → Grund genannt; nicht freigegebenes Boot → beanstandet; unbekannte Mitgliedsnummer → beanstandet (`cargo test`)
- [ ] 13.3 Mindestbesatzung in `check_reservation` umsetzen; Tests mit `example`: Folkeboot Heimrevier mit Skipper + 1 Crew → `Ok`; Trigon außerhalb nur Skipper + Co-Skipper → `TooFewPersons { actual: 2, min: 3 }`; Regel verlangt Co-Skipper, keiner eingetragen → `CoSkipperMissing`; 2 Mitglieder verlangt, Skipper + Gast → `TooFewMembers`; Crewwechsel (Clara Woche 1, David Woche 2, Mindestbesatzung 3) → keine Beanstandung, da alle Eingetragenen zählen; Jolle außerhalb → `AreaNotReleased` (`cargo test`)

## 14. Tourgebühren

- [ ] 14.1 `TourRate::fee(days)` und die Tourtage-Berechnung implementieren; Tests: Freitag 16:00 – Sonntag 18:00 → 3 Tage; Crewwechsel 1.–7./7.–14. Juli → 7 und 8 Tage; ordentlich 10 Tage → 106 €; außerordentlich 15 Tage → 280 €; studentisch 3 Tage → 24 €; Gast 7 Tage → 240 € (`cargo test`)
- [ ] 14.2 `ClubRecord::tour_fee_claims(reservation)` implementieren; Tests: Reservierung ohne `tour` → keine Forderungen; Tour mit Skipper (ordentlich), Crew (Kind → studentisch, Ehrenmitglied → ordentlich) und Gast ohne Einladenden → je Person eine Forderung, Gast 3 Tage = 90 € auf dem Skipper-Konto mit Gastnamen im Text; Gast mit `invited_by` → auf dessen Konto; Tourende 14.07. → fällig 21.07.; zweiter Lauf erzeugt keine Doppelungen (`cargo test`)
- [ ] 14.3 `PaymentStatus` und `ClubRecord::tour_fee_overview(reservation)` implementieren; Test: Anna hat bezahlt, Ben nicht, Clara teilweise → `Paid` / `Open` / `Partial` mit Restbetrag (`cargo test`)

## 15. Integration

- [ ] 15.1 Integrationstest in `src/model/mod.rs`: vollständig befüllte `ClubRecord` aus `example` (Mitglieder aller Arten, Ämter, alle Boote mit Freigaben und einer Sperre, Abnahmen, Widerruf, bestätigte und unbestätigte Einträge beider Formen, Übertrag, Abgeltung, Saison, Beitrags-, Tour- und Abgeltungsforderungen, eine stornierte Forderung, Einzahlungen inkl. Überzahlung, Reservierung, Termin, Aufgabe) übersteht den JSON-Hin- und Rückweg gleich, `validate()` ist `Ok`, `outstanding` liefert die erwarteten Außenstände und eine regelkonforme Dehler-Tour außerhalb liefert bei `check_reservation` `Ok`; verifizieren mit `cargo test`, `cargo clippy -- -D warnings` und `cargo fmt --check` ohne Befunde
- [ ] 15.2 Die vollständige Beispielakte als `src/model/testdata/vereinsakte_beispiel.json` ablegen (lesbar formatiert) und per `include_str!` testen, dass sie sich einlesen lässt, `validate()` `Ok` ist und sie unverändert zurückgeschrieben wird – die Datei dokumentiert zugleich das JSON-Format; verifizieren mit `cargo test`

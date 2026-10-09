# Design

## Context

Greenfield: Das Crate `vereinsakte-rust` (Edition 2024) enthält nur `src/main.rs` mit „Hello, world!“ und hat keine Abhängigkeiten. Motivation und Umfang: siehe proposal.md. Die fachlichen Anforderungen stehen in `specs/*/spec.md`. Quellen der Vereinsregeln sind die Satzung (04.04.2019), die Beitragsordnung und die Segelordnung (14.04.2026) der Segelgruppe an der FH Kiel.

Konvention: Bezeichner, Datentypen, Code und damit auch JSON-Feldnamen sind **englisch**; Doku-Kommentare, Specs und Fehlertexte für Nutzer (`Display`) sind **deutsch**. Fachliche Kürzel (Scheinarten) bleiben deutsch.

## Goals / Non-Goals

**Goals:**
- Reine Datentypen plus seiteneffektfreie Prüf- und Berechnungsfunktionen, ohne I/O testbar.
- Bootsspezifische Regeln der Segelordnung und Gebührensätze der Beitragsordnung als **Daten**, damit eine neue Ordnung oder ein neues Boot keinen Codeänderungsbedarf erzeugt.
- Ein JSON-Dokument (die Vereinsakte) als Grundlage für eine spätere Persistenz.

**Non-Goals:**
- Keine Ablage/kein Repository, keine Indizes: Abfragen laufen linear über die Listen der Vereinsakte (wenige hundert Mitglieder).
- Keine Builder, keine Änderungshistorie, keine Zugriffsrechte, keine Sicherstellung referentieller Integrität beim Einlesen.
- Keine Finanzbuchhaltung (doppelte Buchführung, Vereinskonten, Ausgaben, Abschluss).

## Decisions

### Glossar Fachbegriff → Bezeichner

| Fachbegriff | Typ / Feld |
|---|---|
| Vereinsakte | `ClubRecord` |
| Mitglied, Mitgliedsnummer | `Member`, `MemberId` (Feld `id`) |
| Mitgliedsart | `MembershipType { Regular, Student, Extraordinary, Honorary, ChildOfMember, Erasmus }` |
| Mitgliedschaft (Eintritt, vorläufig, befristet, Austritt, Ausschluss) | `Membership { joined_at, provisional_until, limited_until, left_at, expelled_at }` |
| Anschrift, Telefonnummern | `Address { street, postal_code, city }`, `PhoneNumbers { landline, business, mobile }` |
| Meldeadresse gültig | `address_valid` |
| Kind von, Nutzungszustimmung des Vorstands | `child_of`, `board_consent` |
| Segelschein(art), Funklizenz(art) | `SailingLicense`/`SailingLicenseType`, `RadioLicense`/`RadioLicenseType` |
| Boot, Bootstyp (Jolle, Kielboot) | `Boat`, `BoatType { Dinghy, Keelboat }` |
| Fahrtgebiet (Heimrevier, außerhalb) | `SailingArea { HomeWaters, Outside }` |
| Besatzungsregel, Mindestbesatzung | `CrewRule { area, min_members, min_persons, co_skipper_required, co_skipper_requirement }` |
| Anforderung, „einer von“, Nachweis | `Requirement`, `AnyOf`, `Qualification` |
| Saisonfreigabe, Sperre | `releases: Vec<Release>`, `Block { start, end, reason, by, lifted_by }` |
| Bootserlaubnis, Bootsabnahme, Widerruf | `BoatPermit` (abgeleitet), `BoatCheckout { result: CheckoutResult { Passed, Failed }, examiner }`, `Revocation { by }` |
| Arbeitseinsatz, Umfang, Bezug | `WorkEntry`, `WorkExtent { Amount, Period }`, `WorkReference { WorkEvent, Task }` |
| Stundenübertrag | `HourTransfer` |
| Arbeitstermin, Anmeldung | `WorkEvent`, `registrations` |
| Aufgabe, Status | `Task`, `TaskStatus { Open, InProgress, Done }` |
| Saison, Richtsatz, Stundensatz | `Season { year, hours_quota, compensation_rate, fees }` |
| Jahresnachweis, reduziertes Soll, Abgeltung | `AnnualRecord`, `Reduction`, `Compensation` |
| Segelerlaubnis | `SailingPermit` (abgeleitet) |
| Eignung (Skipper/Co-Skipper) | `Eligibility` |
| Reservierung, Crewplatz, Gast | `Reservation`, `CrewSeat`, `Guest` |
| Beanstandung | `ReservationIssue` |
| Mitgliedskonto, Forderung, Einzahlung, Storno | `MemberAccount`, `Claim`, `Payment`, `Cancellation` |
| Forderungsart, Zahlstatus einer Forderung | `ClaimKind`, `ClaimStatus` |
| Gebührentabelle, Toursatz, Tarifgruppe | `FeeSchedule`, `TourRate`, `TariffGroup { Regular, Student, Extraordinary, Guest }` |
| Zeitpunkt, Geldbetrag | `Timestamp`, `Money` |
| Amt, Rolle | `Office { role: Role, member, start, end }` |
| 1./2. Vorsitzender, Kassenwart, Schriftwart, Oberbootswart, Revisor, Bootswart, Abnehmer | `Role::{Chair, ViceChair, Treasurer, Secretary, HeadBoatwarden, Auditor, Boatwarden(BoatId), Examiner(BoatId)}` |
| Bestätigung, Freigabe | `Confirmation { by, at }`, `Release { season, by, at }` |

### Modulstruktur
`src/model/mod.rs` mit Untermodulen:
`ids`, `time`, `money`, `member`, `office`, `boat`, `requirement`, `boat_permit`, `work_hours`, `sailing_permit`, `reservation`, `work_event`, `task`, `account`, `fees`, `tour_fees`, `club_record`, `issues` sowie `#[cfg(test)] example` (Testdaten der Segelgruppe). `main.rs` deklariert `#[allow(dead_code)] mod model;`, bis die Anwendung das Modell nutzt.
*Alternative:* sofort `lib.rs` – erst sinnvoll mit zweitem Binary oder externen Integrationstests.

### Wurzelobjekt `ClubRecord`
```rust
pub struct ClubRecord {
    members: Vec<Member>, boats: Vec<Boat>, seasons: Vec<Season>,
    annual_records: Vec<AnnualRecord>, boat_checkouts: Vec<BoatCheckout>,
    revocations: Vec<Revocation>, work_entries: Vec<WorkEntry>,
    hour_transfers: Vec<HourTransfer>, reservations: Vec<Reservation>,
    work_events: Vec<WorkEvent>, tasks: Vec<Task>, offices: Vec<Office>,
}
```
Alle Felder `pub`, `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Regeln, die Daten aus mehreren Listen brauchen (Bootserlaubnis, Stunden, Segelerlaubnis, Eignung, Reservierungsprüfung, Salden, Forderungsanlage), sind Methoden auf `ClubRecord`. Regeln, die nur ein Objekt betreffen (Plausibilität, Überschneidung, Anmeldung, aktiv?), sind Methoden des Objekts. `ClubRecord::validate() -> Result<(), Vec<RecordIssue>>` prüft Akte-weite Invarianten: eindeutige Mitgliedsnummern, Satzungsregeln für Vorstandsämter und die Berechtigung aller gespeicherten Handlungen (siehe Ämter und Rechte).
*Alternative:* freie Funktionen mit vielen Slice-Parametern – wird mit Abnahmen, Überträgen, Saisons und Ämtern unhandlich.

### Zeitpunkte: `chrono::DateTime<FixedOffset>`
`pub type Timestamp = DateTime<FixedOffset>;`. Mit chrono-Feature `serde` wird RFC3339 geschrieben, der Offset bleibt erhalten, Eingaben ohne Offset oder reine Daten scheitern. Jahres- und Tageszuordnung über `year()` / `date_naive()` im gespeicherten Offset (lokale Vereinszeit).
*Alternativen:* `DateTime<Utc>` verliert den Offset; `jiff`/`time` möglich, chrono ist verbreiteter.

Zeitraumgrenzen (`left_at`, `expelled_at`, `limited_until`, Sperrende) sind **exklusive Endzeitpunkte**: aktiv/gesperrt gilt für `start <= t < end`.

### Kennungen
- **Mitglied:** `#[serde(try_from = "u32", into = "u32")] pub struct MemberId(NonZeroU32)` – die Mitgliedsnummer ist die Kennung; `NonZeroU32` schließt 0 aus, `u32` negative Werte, serde lehnt Zeichenketten ab. Das Feld im `Member` heißt wie bei allen Fachobjekten `id` (JSON `"id": 42`). Verweise (Crew, Abnahmen, Konten …) enthalten im JSON die Zahl, z. B. `"skipper": 42`. `Display` gibt die Zahl ohne führende Nullen aus. Die Nummer ist laut Beitragsordnung auch die SEPA-Mandatsreferenz und darf nicht nachträglich geändert werden. Eindeutigkeit prüft `ClubRecord::validate()`.
- **Alle anderen:** `BoatId`, `OfficeId`, `CheckoutId`, `RevocationId`, `WorkEntryId`, `HourTransferId`, `ReservationId`, `WorkEventId`, `TaskId`, `ClaimId`, `PaymentId` per Makro als `#[serde(transparent)] struct X(Uuid)` mit `new()` (v4).
Alle Kennungen `Clone, Eq, Hash, Ord`; UUID-Typen zusätzlich `Copy`.
*Alternative:* UUID auch für Mitglieder – vom Nutzer verworfen; die Nummer ist im Verein ohnehin der eindeutige Schlüssel.

### Derives und Enum-Darstellung
Alle Typen `Debug, Clone, PartialEq, Serialize, Deserialize`. Optionale Felder `#[serde(default, skip_serializing_if = "Option::is_none")]`, Listen analog mit `Vec::is_empty`, `bool`-Felder mit Default `false` analog. Auswahl-Enums (`MembershipType`, `BoatType`, `SailingArea`, `CheckoutResult`, `TaskStatus`, `TariffGroup`) mit `#[serde(rename_all = "snake_case")]`, also z. B. `"child_of_member"`, `"failed"`, `"outside"`.

### Scheinarten als einfache Zeichenkette
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum SailingLicenseType { SbfSee, SbfBinnen, Sks, Sss, Shs, Br, Other(String) }
```
`impl From<SailingLicenseType> for String` liefert `"SBF See"`, `"SBF Binnen"`, `"SKS"`, `"SSS"`, `"SHS"`, `"BR"` bzw. den Freitext; `impl From<String>` vergleicht getrimmt und ohne Beachtung der Groß-/Kleinschreibung und fällt sonst auf `Other` zurück. `Display` nutzt dieselbe Abbildung. Gleiches Muster für `RadioLicenseType { Src, Lrc, Ubi, Roc, Other(String) }`.
`Other("SKS")` würde nach dem Rückweg zu `Sks`; deshalb ist `SailingLicenseType::from(&str)` der vorgesehene Konstruktor, und ein Test sichert die Normalisierung ab.
*Alternative:* serde-Standard (`{"Other": "…"}`) – vom Nutzer abgelehnt; reiner `String` – verliert Prüfbarkeit.

Ein Schein ist `SailingLicense { kind, acquired_at: Option<Timestamp> }`, eine Funklizenz `RadioLicense { kind, acquired_at }`.

### Mitglied
```rust
Member { id: MemberId, last_name, first_name, address: Address,
  phone: PhoneNumbers, email: Option<String>,
  address_valid: bool /* serde default = true */,
  membership_type: MembershipType, child_of: Option<MemberId>, board_consent: bool,
  membership: Membership, sailing_licenses: Vec<SailingLicense>, radio_licenses: Vec<RadioLicense>,
  account: MemberAccount }
Membership { joined_at: Timestamp, provisional_until, limited_until, left_at, expelled_at: Option<Timestamp> }
```
`MembershipType::{pays_fee, owes_hours, tariff_group}` liefern die Tabellen aus den Specs. `Member::is_active_at(t)`, `is_active_in(year)`, `is_provisional_at(t)`, `declare_resignation(declared_at)` (setzt `left_at` auf den 1.1. des Folgejahres, 00:00, im Offset von `declared_at`; Satzung §7, Geschäftsjahr = Kalenderjahr), `has_sailing_license`, `has_radio_license`.

### Boot und Anforderungen
```rust
Boat { id, name, boat_type: BoatType, class: Option<String>, year_built: Option<u16>,
  length_m, beam_m, draft_m, sail_area_m2: Option<f64>, berths: Option<u8>,
  engine, mooring: Option<String>,
  skipper_requirement: Requirement,
  crew_rules: Vec<CrewRule>,        // höchstens eine je Fahrtgebiet
  releases: Vec<Release>,           // Saisonfreigaben
  blocks: Vec<Block> }
Release { season: i32, by: MemberId, at: Timestamp }
CrewRule { area: SailingArea, min_members: u8, min_persons: u8,
  co_skipper_required: bool, co_skipper_requirement: Requirement }
Block { start: Timestamp, end: Option<Timestamp>, reason: String, by: MemberId, lifted_by: Option<MemberId> }

#[serde(transparent)] Requirement(Vec<AnyOf>)   // alle Gruppen
#[serde(transparent)] AnyOf(Vec<Qualification>) // mindestens einer
#[serde(rename_all = "snake_case")]
enum Qualification { SailingLicense(SailingLicenseType), RadioLicense(RadioLicenseType),
                     BoatPermit(BoatId), BoatPermitForType(BoatType) }
```
JSON einer Qualification: `{"sailing_license": "SKS"}`, `{"boat_permit": "<uuid>"}`, `{"boat_permit_for_type": "dinghy"}`. Die Bootserlaubnis für das eigene Boot wird für den Skipper immer geprüft und steht nicht in den Daten – sie kann nicht vergessen werden.
`ClubRecord::check_requirement(member, &Requirement, t) -> Result<(), Vec<AnyOf>>` liefert die nicht erfüllten Gruppen. `Boat::availability_at(t) -> Result<(), Vec<BoatUnavailable>>` prüft Freigabe für `t.year()` und Sperren.
*Alternative:* Regeln je Bootsklasse fest im Code (`match class`) – jede Änderung der Segelordnung wäre eine Codeänderung.

### Bootserlaubnis
`BoatCheckout { id, member, boat, at, result: CheckoutResult, examiner: MemberId, remark }`, `Revocation { id, member, boat, at, reason, by: MemberId }`.
`ClubRecord::boat_permit(member, boat, t) -> Result<(), PermitMissing>`: letzte bestandene Abnahme mit `at <= t` und letzten Widerruf mit `at <= t` bestimmen; gültig, wenn die Abnahme existiert und kein Widerruf nach oder zeitgleich mit ihr liegt. Danach `address_valid` prüfen (`PermitMissing::{NoCheckout, Revoked, AddressInvalid}`). Ruhen ist ein abgeleiteter Zustand, daher lebt die Erlaubnis ohne Datenänderung wieder auf.

### Arbeitsstunden
```rust
WorkEntry { id, member, extent: WorkExtent, subject: WorkSubject, description: Option<String>, reference: Option<WorkReference>,
            confirmation: Option<Confirmation> }
Confirmation { by: MemberId, at: Timestamp }
enum WorkExtent { Amount { date: Timestamp, hours: f64 }, Period { start: Timestamp, end: Timestamp } }
#[serde(rename_all = "snake_case")] enum WorkReference { WorkEvent(WorkEventId), Task(TaskId) }
HourTransfer { id, year: i32, from: MemberId, to: MemberId, hours: f64, description }
```
`#[serde(rename_all = "snake_case")] enum WorkSubject { Boat(BoatId), Club }` ist Pflichtfeld (kein Default), JSON `{"boat": "<uuid>"}` oder `"club"`. Ein Einsatz braucht weder Termin noch Aufgabe; `ClubRecord::validate_work_entry(&entry)` meldet einen Widerspruch, wenn `reference` auf einen Termin oder eine Aufgabe mit Boot zeigt und `subject` ein anderes Boot oder `Club` ist.
`WorkExtent` steht im JSON **flach** im Eintrag (`date`+`hours` oder `start`+`end`). Dafür nutzt `WorkEntry` `#[serde(try_from = "RawWorkEntry", into = "RawWorkEntry")]`; `RawWorkEntry` hat die vier Umfangsfelder als `Option` (übrige Felder wie im Eintrag), und `TryFrom` lehnt „beide Formen“ und „keine Form“ ab. Ein `#[serde(untagged)]`-Enum mit `flatten` würde überzählige Felder stillschweigend ignorieren und ist deshalb verworfen. `WorkReference` als Enum macht „Termin und Aufgabe zugleich“ unmöglich.
`WorkEntry::hours()`, `::year()`, `::validate()`; `ClubRecord::credited_hours(member, year)` = **bestätigte** Einsätze + erhaltene − abgegebene Überträge + abgegoltene Stunden; `work_entries_of(member, year)` liefert alle Einsätze inkl. unbestätigter; `ClubRecord::hours_for_task(task)`.

### Stunden als `f64`
Bruchteile (2,5 h) sind gefordert; Vergleich Ist ≥ Soll mit Toleranz `1e-9`.

### Geld: `Money` auf Basis von `rust_decimal`
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(Decimal);   // Euro, immer auf 2 Nachkommastellen normalisiert
```
- Konstruktion nur über `Money::from_euros(Decimal)`, `Money::from_cents(i64)`, `Money::ZERO` und `FromStr`; jede Konstruktion und jede Rechenoperation normalisiert per `round_dp_with_strategy(2, MidpointAwayFromZero)` (kaufmännische Rundung), sodass ein `Money` nie mehr als 2 Nachkommastellen trägt.
- `Add`, `Sub`, `Neg`, `Sum`; `Mul<Decimal>` für Satz × Menge (z. B. Stundensatz × 2,5 h) und `Money::prorate(numerator, denominator)` für den anteiligen Beitrag – beide runden auf Cent.
- **serde**: eigene `Serialize`/`Deserialize`-Implementierung, damit die Darstellung unabhängig von `rust_decimal`-Features ist: Schreiben als String mit genau 2 Nachkommastellen (`"240.00"`); Lesen nur aus Strings, mehr als 2 Nachkommastellen → Fehler (statt stiller Rundung).
- `Display` deutsch: „240,00 €“, „-3,50 €“.
- Abhängigkeit: `rust_decimal` (ohne serde-Feature, da eigene Implementierung); in Tests `rust_decimal_macros::dec!` als dev-dependency.
*Alternativen:* `Money(i64)` – exakt und abhängigkeitsfrei, aber im JSON unlesbar (`24000`); `rusty-money` – keine serde-Unterstützung, kaum gepflegt; `f64` – Rundungsfehler in Salden.

### Saison und Gebührentabelle
```rust
Season { year, hours_quota: Option<f64>, compensation_rate: Option<Money>, fees: Option<FeeSchedule> }
FeeSchedule {
  annual_fee: Vec<TypeRate>,       // je Mitgliedsart
  admission_fee: Vec<TypeRate>,    // je Mitgliedsart
  tour: Vec<TourRate>,             // je Tarifgruppe
}
TypeRate { membership_type, amount: Money }
TourRate { tariff_group, day: Money, week: Money, first_week: Option<Money> }
```
`FeeSchedule::default()` enthält die Sätze der Beitragsordnung (siehe Specs `beitraege`, `tourgebuehren`); fehlende Saison/Tabelle → Default. Listen statt Maps, damit das JSON lesbar bleibt (`[{"membership_type": "regular", "amount": 24000}, …]`).

### Mitgliedskonto
```rust
// im Member: #[serde(default, skip_serializing_if = "MemberAccount::is_empty")] account: MemberAccount
MemberAccount { claims: Vec<Claim>, payments: Vec<Payment> }
Claim { id: ClaimId, kind: ClaimKind, amount: Money, text: String,
        created_at: Timestamp, due_at: Option<Timestamp>, cancelled: Option<Cancellation> }
Cancellation { at: Timestamp, reason: String }
#[serde(rename_all = "snake_case")]
enum ClaimKind { AnnualFee { year: i32 }, AdmissionFee,
                 TourFee { reservation: ReservationId, guest: Option<String> },
                 WorkCompensation { year: i32 }, Other }
Payment { id: PaymentId, amount: Money, at: Timestamp, text: Option<String> }
```
JSON einer Art: `{"annual_fee": {"year": 2026}}`, `"admission_fee"`, `{"tour_fee": {"reservation": "<uuid>", "guest": "Max Gast"}}`.
- Das Konto gehört zum Mitglied (Nutzerwunsch: „ein Objekt mit zwei Attributen je Mitglied“); es gibt keine Vereinskonten und keine Soll/Haben-Buchungen.
- `Claim::validate()`/`Payment::validate()` prüfen Betrag > 0. Korrekturen erfolgen über `cancel(at, reason)` statt Löschen, damit nachvollziehbar bleibt, was gefordert war.
- Berechnungen auf `MemberAccount`: `total_claimed()` (ohne stornierte), `total_paid()`, `balance()` = Differenz, `claim_status() -> Vec<ClaimStatus { claim_id, amount, open: Money }>` (bezahlt = `open == 0`, teilweise = `0 < open < amount`). Für `claim_status` werden die nicht stornierten Forderungen nach (`due_at` oder `created_at`, `created_at`, `id`) sortiert und die Summe aller Einzahlungen in dieser Reihenfolge verteilt; damit gilt Σ offen = max(Kontostand, 0). Eine Zuordnung von Einzahlungen zu Forderungen wird **nicht gespeichert** – nur die Summe zählt.
- `ClubRecord::outstanding(t) -> Vec<Outstanding { member, open, overdue }>` für alle Mitglieder mit offenem Betrag.
- Forderungen werden idempotent angelegt: `MemberAccount::has_claim(&ClaimKind)` prüft, ob bereits eine Forderung gleicher Art existiert, **auch eine stornierte**, damit eine bewusst stornierte Forderung nicht beim nächsten Lauf neu entsteht.
*Alternativen:* doppelte Buchführung mit Vereins- und Ertragskonten (erster Entwurf) – vom Nutzer als zu aufwendig verworfen; explizite Zuordnung Einzahlung → Forderung – vom Nutzer verworfen („die Summe muss stimmen“).

### Beiträge
`ClubRecord::annual_fee_claims(year) -> Vec<(MemberId, Claim)>` erzeugt für alle Mitglieder, die im Jahr aktiv und beitragspflichtig sind und noch keine Forderung `ClaimKind::AnnualFee { year }` haben, die Forderung; `admission_fee_claims()` analog. Die Methoden **liefern** Forderungen, `ClubRecord::add_claims(Vec<(MemberId, Claim)>)` trägt sie ein – so sind sie als Vorschau nutzbar.
- Anteil im Eintrittsjahr: `rate * (13 - join_month) / 12`, gerundet; Erasmus-Pauschale nicht anteilig.
- Fälligkeit: `annual_fee_due_date(year)` = 1. April, verschoben auf den nächsten Tag, der kein Samstag, Sonntag, Karfreitag oder Ostermontag ist (Osterdatum per Gauß-Algorithmus); bei späterem Eintritt der Eintritt. Offset `+02:00` (MESZ) als Konstante der Vereinszeit.
- `ClubRecord::fee_arrears(member, year) -> Option<Money>` für die Segelerlaubnis: offene Beträge aller Jahresbeitragsforderungen bis `year` aus `claim_status`; fehlt bei Beitragspflicht die Forderung für `year`, gilt der volle Satz als Rückstand.

### Tourgebühren
`Reservation` hat `tour: bool`, `crew: Vec<CrewSeat { member, from: Option<Timestamp>, until: Option<Timestamp> }>` und `guests: Vec<Guest { name, phone, invited_by: Option<MemberId>, from, until }>`. Skipper und Co-Skipper sind die ganze Reservierung an Bord.
- `Reservation::period_of(person) -> (Timestamp, Timestamp)`; Tourtage = `(until.date_naive() - from.date_naive()).num_days() + 1`.
- `TourRate::fee(days)`: `w = days / 7`, `r = days % 7`; ohne `first_week`: `w * week + r * day`; mit `first_week`: `(w >= 1) * first_week + (w - 1).max(0) * week + r * day`.
- `ClubRecord::tour_fee_claims(reservation) -> Vec<(MemberId, Claim)>`: je Person an Bord eine Forderung (Gäste auf `invited_by` oder Skipper, Text nennt den Gast), fällig `until + 7 Tage`, `ClaimKind::TourFee`, ohne Doppelungen (Mitglied: gleiche Reservierung ohne Gast; Gast: gleiche Reservierung und gleicher Gastname).
- `ClubRecord::tour_fee_overview(reservation) -> Vec<TourFeeLine { person, amount, status: PaymentStatus }>` mit `PaymentStatus::{Paid, Partial(Money), Open}` aus `claim_status`.
- Mindestbesatzung zählt jede eingetragene Person einmal, unabhängig von eigenen Zeiträumen (bewusste Entscheidung, siehe Risiken).

### Segelerlaubnis
`AnnualRecord { member, year, reduced_quota: Option<Reduction { hours, reason }>, compensation: Option<Compensation { hours, amount: Option<Money>, approved_at, reason }> }`.
`ClubRecord::required_hours(member, year)`, `::sailing_permit(member, year) -> Result<(), Vec<SailingPermitMissing>>` mit `SailingPermitMissing::{Inactive, FeeArrears(Money), HoursMissing { missing }, NoBoardConsent}`; `sailing_permit_at(member, t)` = `is_active_at(t)` + `sailing_permit(member, t.year())`, damit ein im März ausgeschiedenes Mitglied im Juni nicht mitsegelt. `compensation_claims()` erzeugt Forderungen zu genehmigten Abgeltungen (fällig `approved_at`).

### Eignung und Reservierungsprüfung
`ClubRecord::skipper_eligibility(member, boat, t)` und `::co_skipper_eligibility(member, boat, area, t)` → `Result<(), Vec<EligibilityIssue>>` mit `EligibilityIssue::{NoSailingPermit(Vec<SailingPermitMissing>), BoatPermit(PermitMissing), QualificationMissing(AnyOf), AreaNotReleased}`.
`Reservation { id, boat, start, end, area, tour, skipper, co_skipper: Option<MemberId>, crew: Vec<CrewSeat>, guests: Vec<Guest>, remark }`; `::validate()` (inkl. Zeiträume an Bord innerhalb der Reservierung), `::overlaps(&other)`.
`ClubRecord::check_reservation(&r) -> Result<(), Vec<ReservationIssue>>` sammelt Plausibilität, unbekannte Mitglieder/Boote, `BoatUnavailable`, fehlende Segelerlaubnis je Mitglied, Skipper-/Co-Skipper-Eignung und Besatzungsregel (`AreaNotReleased`, `CoSkipperMissing`, `TooFewMembers { actual, min }`, `TooFewPersons { actual, min }`). Alle Fehlerenums implementieren `Display` mit deutschem Text; keine Fehler-Crate.

### Ämter und Rechte
```rust
Office { id, role: Role, member: MemberId, start: Timestamp, end: Option<Timestamp> }
#[serde(rename_all = "snake_case")]
enum Role { Chair, ViceChair, Treasurer, Secretary, HeadBoatwarden, Auditor,
            Boatwarden(BoatId), Examiner(BoatId) }
```
JSON: `"chair"`, `{"boatwarden": "<uuid>"}`. Zeitverhalten: `ClubRecord::appoint(member, role, start) -> OfficeId` legt ein neues `Office` an; `ClubRecord::resign(member, role, at) -> Result<(), OfficeError>` setzt `end` des zu `at` aktiven Amtes (Fehler, wenn keins aktiv ist oder `at < start`). Ämter werden nie gelöscht – die Liste ist die Historie, Abfragen filtern nach `start <= t < end`. Alle Rechteprüfungen laufen zeitbezogen über diese Abfrage, daher enden Rechte automatisch mit dem Amt. Wer ernennen darf (Mitgliederversammlung, Ersatz durch den Vorstand) wird nicht geprüft. `Role::is_board()` (die fünf Vorstandsämter), `Role::is_legal_representative()` (§ 26 BGB: Chair, Treasurer, Secretary). Abfragen: `ClubRecord::holders(role, t)`, `boatwardens(boat, t)`, `board(t)`, `holds(member, pred, t)`.
`MembershipType::has_voting_right()` (Regular, Student, Honorary) plus „aktiv und nicht vorläufig“ ergibt die Stimmberechtigung (§4, §5).

**Rechte** sind eine reine Funktion `ClubRecord::authorize(actor, &Action, t) -> Result<(), NotAuthorized>` mit
`Action::{BlockBoat(BoatId), LiftBlock(BoatId), ReleaseBoat(BoatId), ConfirmWork(WorkEntryId), RecordCheckout(BoatId), Revoke(BoatId)}`:

| Action | erlaubt für |
|---|---|
| BlockBoat, LiftBlock, ReleaseBoat | Boatwarden(boot), HeadBoatwarden, Vorstand |
| ConfirmWork | `subject = Boat(boot)`: Boatwarden(boot), HeadBoatwarden, Vorstand; `subject = Club`: jedes zum Zeitpunkt aktive Amt; nie `actor == entry.member` |
| RecordCheckout | Examiner(boot), Boatwarden(boot), Vorstand |
| Revoke | Vorstand |

Mutierende Methoden prüfen zuerst und ändern nur bei Erfolg: `block_boat(actor, boat, start, end, reason)`, `lift_block(actor, boat, end)`, `release_boat(actor, boat, season, t)`, `confirm_work(actor, entry, t)`, `record_checkout(actor, checkout)`, `revoke(actor, revocation)`. `validate()` ruft `authorize` für jede gespeicherte Handlung mit deren Urheber und Zeitpunkt erneut auf und meldet Verstöße als `RecordIssue::Unauthorized { … }`; ebenso `BoardMemberWithoutVote`, `BoardOfficesAccumulated`, `BoardOfficeTwice`.
*Alternative:* Rechte nur in einer späteren Anwendungsschicht – dann wären Bestandsdaten nicht prüfbar und jede Oberfläche müsste die Regeln duplizieren.

### Arbeitstermine und Aufgaben
`WorkEvent { id, title, start, end, location, description, boat: Option<BoatId>, max_participants: Option<u32>, registrations: Vec<MemberId> }` mit `validate`, `register -> Result<(), RegistrationError>`, `unregister`.
`Task { id, title, description, boat, assignees: Vec<MemberId>, estimated_hours: Option<f64>, due_at: Option<Timestamp>, status: TaskStatus, done_at: Option<Timestamp> }` mit `assign`, `unassign`, `set_status(status, t)`.

### Testdaten der Segelgruppe (`example`)
Testmodul mit den fünf Booten nach Segelordnung 14.04.2026; „SKS+“ = [SKS | SSS | SHS | BR], „Segelkenntnisse“ = [Bootserlaubnis Jolle | SBF See | SKS | SSS | SHS | BR]:

| Boot | Typ/Klasse | Skipper-Anforderung | Heimrevier | Außerhalb |
|---|---|---|---|---|
| Filou | Jolle | – | 1 Mitglied | keine Regel |
| Dipl.Ing.ling | Jolle, Yngling | – | 1 Mitglied | keine Regel |
| Harald Blauzahn | Kielboot, Folkeboot | SKS+ | 2 Mitglieder | 2 Mitglieder, Co-Skipper mit Segelkenntnissen |
| Schruvtrekker | Kielboot, Dehler 31 | SKS+, SBF See, SRC/LRC | 2 Mitglieder, Co-Skipper mit Segelkenntnissen | 2 Mitglieder / 3 Personen, Co-Skipper mit Bootserlaubnis Harald Blauzahn + SBF See |
| Trigon | Kielboot, Matcher 37 | wie Dehler 31 | wie Dehler 31 | wie Dehler 31 |

## Risks / Trade-offs

- [Besatzungsregeln vereinfacht] → Motoreinweisung (Dehler/Matcher), Praxisnachweis für die Matcher-Erlaubnis und „Boote in Rufweite“ bei Jollenschulungen werden nicht modelliert; die Matcher-Erlaubnis entsteht wie jede andere über eine (vom Vorstand erteilte) Abnahme.
- [„Weiteres Mitglied mit Segelkenntnissen“ als Co-Skipper] → Im Heimrevier der Dehler/Matcher wird es als Co-Skipper eingetragen, was Segelordnung 2.4 ohnehin fordert.
- [Mindestbesatzung ignoriert Crewwechsel] → Gezählt werden alle eingetragenen Personen, nicht die gleichzeitig an Bord befindlichen. Nach Entscheidung des Nutzers wird das nicht im Code geprüft: Die erfassten Zeiträume an Bord dokumentieren die Besatzung, und der Vorstand prüft sie bei Bedarf (Segelordnung 2.4 behält ihm die Streichung vor).
- [Soll bei unterjährigem Eintritt nicht anteilig] → Regelt der Vorstand über ein reduziertes Soll.
- [Annahmen zu Mitgliedsarten] → Erasmus ohne Stundenpflicht; Studierende wie ordentliche Mitglieder; Ehrenmitglieder und Erasmus zahlen Tourgebühren wie ordentliche (Nutzerentscheidung). Änderungen betreffen nur `MembershipType`-Methoden bzw. die Gebührentabelle.
- [FIFO-Tilgung über alle Forderungsarten] → Eine Zahlung tilgt zuerst die älteste Forderung, auch wenn sie als Beitrag gemeint war; dadurch kann trotz „Beitrag überwiesen“ ein Beitragsrückstand bestehen, solange ältere Tourgebühren offen sind. Gewollt (nur die Summe zählt) und in der Spec als Szenario festgehalten.
- [Fälligkeit 1. April ohne vollständigen Feiertagskalender] → Nur Wochenende, Karfreitag und Ostermontag; andere Feiertage fallen nie auf Anfang April.
- [Gästegebühren auf Mitgliedskonto] → Der Verein hat keine Konten für Gäste; das einladende Mitglied bzw. der Skipper kassiert.
- [`f64` in Structs] → kein `Eq`/`Hash` für Typen mit Stunden/Maßen; für das Modell unproblematisch.
- [Rechte nur für Fachhandlungen] → Forderungen, Einzahlungen, Mitgliederverwaltung und Ämterpflege prüfen keine Rechte (Wahlen/Mitgliederversammlung nicht im Umfang); kann später über weitere `Action`-Varianten ergänzt werden.
- [Gegenstand vom Mitglied gewählt] → Wer „Verein“ statt eines Bootes angibt, erweitert den Kreis der Bestätigenden; der Bestätigende sieht Gegenstand und Beschreibung und kann die Bestätigung verweigern.
- [Referentielle Integrität] → Verweise auf unbekannte Kennungen werden nicht beim Einlesen, sondern in den Prüfungen gemeldet.

## Migration Plan

Entfällt (keine bestehenden Daten, kein bestehendes Verhalten).

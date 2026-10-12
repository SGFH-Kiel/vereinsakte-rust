//! Testdaten: eine erfundene, aber realistische Vereinsakte der Segelgruppe.
//!
//! Alle Tests bauen auf diesen Daten auf. Namen, Adressen und Mitgliedsnummern
//! sind erfunden; Boote und Regeln entsprechen der Segelordnung vom 14.04.2026.

use uuid::Uuid;

use super::account::{Cancellation, Claim, ClaimKind, MemberAccount, Payment};
use super::boat::{Boat, BoatType, CrewRule, Release, SailingArea};
use super::boat_permit::{BoatCheckout, CheckoutResult, Revocation};
use super::club_record::ClubRecord;
use super::fees::Season;
use super::ids::{
    BoatId, CheckoutId, ClaimId, HourTransferId, MemberId, OfficeId, PaymentId, ReservationId,
    RevocationId, TaskId, WorkEntryId, WorkEventId,
};
use super::member::{
    Address, Member, MembershipType, RadioLicense, RadioLicenseType, SailingLicense,
    SailingLicenseType,
};
use super::money::Money;
use super::office::{Office, Role};
use super::requirement::{AnyOf, Qualification, Requirement};
use super::reservation::{CrewSeat, Reservation};
use super::sailing_permit::{AnnualRecord, Compensation, Reduction};
use super::task::{Task, TaskStatus};
use super::time::{Timestamp, club_time};
use super::work_event::WorkEvent;
use super::work_hours::{
    Confirmation, HourTransfer, WorkEntry, WorkExtent, WorkReference, WorkSubject,
};

/// Zeitpunkt in Vereinszeit (MEZ/MESZ).
pub fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Timestamp {
    club_time(year, month, day, hour, minute)
}

const fn member_id(number: u32) -> MemberId {
    match MemberId::new(number) {
        Some(id) => id,
        None => panic!("Mitgliedsnummer 0"),
    }
}

// --- Mitglieder -----------------------------------------------------------

/// Anna Petersen – erfahrene Seglerin (SKS, SBF See, SRC), Skipperin der Dehler.
pub const ANNA: MemberId = member_id(101);
/// Ben Hansen – Bootswart, SKS.
pub const BEN: MemberId = member_id(102);
/// Clara Jensen – Studentin, SBF See und SKS.
pub const CLARA: MemberId = member_id(103);
/// David Schulz – Student, Webseite, Abnehmer der Filou.
pub const DAVID: MemberId = member_id(104);
/// Eva Möller – Ehrenmitglied und 1. Vorsitzende.
pub const EVA: MemberId = member_id(105);
/// Frank Koch – Oberbootswart.
pub const FRANK: MemberId = member_id(106);
/// Greta Lorenzen – außerordentliches Mitglied ohne Nutzungszustimmung.
pub const GRETA: MemberId = member_id(107);
/// Hannes Petersen – Kind von Anna.
pub const HANNES: MemberId = member_id(108);
/// Inès Dupont – Erasmus, März bis Oktober 2026.
pub const INES: MemberId = member_id(109);
/// Jonas Brandt – Kassenwart.
pub const JONAS: MemberId = member_id(110);
/// Karin Voß – Schriftwartin.
pub const KARIN: MemberId = member_id(111);
/// Lukas Meier – Revisor.
pub const LUKAS: MemberId = member_id(112);
/// Malte Kruse – vorläufig aufgenommener Student.
pub const MALTE: MemberId = member_id(113);

fn kiel(street: &str, postal_code: &str) -> Address {
    Address::new(street, postal_code, "Kiel")
}

fn person(
    id: MemberId,
    last_name: &str,
    first_name: &str,
    address: Address,
    membership_type: MembershipType,
    joined_at: Timestamp,
) -> Member {
    let mut member = Member::new(
        id,
        last_name,
        first_name,
        address,
        membership_type,
        joined_at,
    );
    member.email = Some(format!(
        "{}.{}@example.org",
        first_name.to_lowercase(),
        last_name.to_lowercase()
    ));
    member.login_email = Some(format!(
        "{}.{}@sgfh.example",
        login_name(first_name),
        login_name(last_name)
    ));
    member
}

/// Name in der Schreibweise von Microsoft-Kontonamen (ohne Umlaute und Akzente).
fn login_name(name: &str) -> String {
    name.to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
        .replace('è', "e")
}

fn with_licenses(mut member: Member, sailing: &[&str], radio: &[&str]) -> Member {
    member.sailing_licenses = sailing.iter().map(|&s| SailingLicense::new(s)).collect();
    member.radio_licenses = radio.iter().map(|&s| RadioLicense::new(s)).collect();
    member
}

pub fn anna() -> Member {
    let mut m = person(
        ANNA,
        "Petersen",
        "Anna",
        kiel("Holtenauer Straße 112", "24105"),
        MembershipType::Regular,
        at(2015, 3, 1, 0, 0),
    );
    m.phone.mobile = Some("0151 23456789".into());
    m.phone.landline = Some("0431 556677".into());
    m.sailing_licenses = vec![
        SailingLicense {
            kind: SailingLicenseType::SbfSee,
            acquired_at: Some(at(2014, 6, 20, 0, 0)),
        },
        SailingLicense {
            kind: SailingLicenseType::Sks,
            acquired_at: Some(at(2017, 9, 15, 0, 0)),
        },
    ];
    m.radio_licenses = vec![RadioLicense::new("SRC")];
    m
}

pub fn ben() -> Member {
    let m = person(
        BEN,
        "Hansen",
        "Ben",
        kiel("Knooper Weg 45", "24103"),
        MembershipType::Regular,
        at(2012, 4, 1, 0, 0),
    );
    with_licenses(m, &["SBF See", "SKS"], &[])
}

pub fn clara() -> Member {
    let mut m = person(
        CLARA,
        "Jensen",
        "Clara",
        kiel("Sophienblatt 20", "24114"),
        MembershipType::Student,
        at(2023, 4, 1, 0, 0),
    );
    m.phone.mobile = Some("0176 99887766".into());
    with_licenses(m, &["SBF See", "SKS"], &[])
}

pub fn david() -> Member {
    person(
        DAVID,
        "Schulz",
        "David",
        kiel("Westring 280", "24116"),
        MembershipType::Student,
        at(2024, 10, 1, 0, 0),
    )
}

pub fn eva() -> Member {
    let m = person(
        EVA,
        "Möller",
        "Eva",
        kiel("Düsternbrooker Weg 70", "24105"),
        MembershipType::Honorary,
        at(1988, 5, 1, 0, 0),
    );
    with_licenses(m, &["SBF See", "SKS", "SSS"], &["LRC"])
}

pub fn frank() -> Member {
    let m = person(
        FRANK,
        "Koch",
        "Frank",
        kiel("Feldstraße 99", "24105"),
        MembershipType::Regular,
        at(2005, 2, 1, 0, 0),
    );
    with_licenses(m, &["SBF See", "SKS"], &["SRC"])
}

pub fn greta() -> Member {
    person(
        GRETA,
        "Lorenzen",
        "Greta",
        kiel("Schönberger Straße 50", "24148"),
        MembershipType::Extraordinary,
        at(2020, 1, 1, 0, 0),
    )
}

pub fn hannes() -> Member {
    let mut m = person(
        HANNES,
        "Petersen",
        "Hannes",
        kiel("Holtenauer Straße 112", "24105"),
        MembershipType::ChildOfMember,
        at(2022, 5, 1, 0, 0),
    );
    m.email = None;
    m.login_email = None;
    m.child_of = Some(ANNA);
    m
}

pub fn ines() -> Member {
    let mut m = person(
        INES,
        "Dupont",
        "Inès",
        kiel("Olshausenstraße 40", "24118"),
        MembershipType::Erasmus,
        at(2026, 3, 1, 0, 0),
    );
    m.membership.limited_until = Some(at(2026, 11, 1, 0, 0));
    m
}

pub fn jonas() -> Member {
    let m = person(
        JONAS,
        "Brandt",
        "Jonas",
        kiel("Elisabethstraße 8", "24143"),
        MembershipType::Regular,
        at(2010, 9, 1, 0, 0),
    );
    with_licenses(m, &["SBF See"], &[])
}

pub fn karin() -> Member {
    let m = person(
        KARIN,
        "Voß",
        "Karin",
        kiel("Lornsenstraße 21", "24105"),
        MembershipType::Regular,
        at(2016, 4, 1, 0, 0),
    );
    with_licenses(m, &["SBF See", "BR"], &[])
}

pub fn lukas() -> Member {
    person(
        LUKAS,
        "Meier",
        "Lukas",
        kiel("Ringstraße 12", "24114"),
        MembershipType::Regular,
        at(2018, 3, 1, 0, 0),
    )
}

pub fn malte() -> Member {
    let mut m = person(
        MALTE,
        "Kruse",
        "Malte",
        kiel("Wilhelminenstraße 5", "24103"),
        MembershipType::Student,
        at(2026, 5, 15, 0, 0),
    );
    m.membership.provisional_until = Some(at(2027, 4, 30, 0, 0));
    m
}

pub fn members() -> Vec<Member> {
    vec![
        anna(),
        ben(),
        clara(),
        david(),
        eva(),
        frank(),
        greta(),
        hannes(),
        ines(),
        jonas(),
        karin(),
        lukas(),
        malte(),
    ]
}

// --- Boote (Segelordnung vom 14.04.2026) ----------------------------------

pub const FILOU: BoatId = BoatId::from_uuid(Uuid::from_u128(0xb0a7_0001));
pub const DIPL_ING_LING: BoatId = BoatId::from_uuid(Uuid::from_u128(0xb0a7_0002));
pub const HARALD_BLAUZAHN: BoatId = BoatId::from_uuid(Uuid::from_u128(0xb0a7_0003));
pub const SCHRUVTREKKER: BoatId = BoatId::from_uuid(Uuid::from_u128(0xb0a7_0004));
pub const TRIGON: BoatId = BoatId::from_uuid(Uuid::from_u128(0xb0a7_0005));

/// „Mindestens SKS“: SKS, SSS, SHS oder der ältere BR-Schein.
pub fn sks_or_higher() -> AnyOf {
    use SailingLicenseType::*;
    AnyOf::sailing_licenses(&[Sks, Sss, Shs, Br])
}

/// „Segelkenntnisse“ (z. B. Jollenerlaubnis, SBF See, SKS bzw. BR-Schein).
pub fn sailing_skills() -> AnyOf {
    use SailingLicenseType::*;
    AnyOf(vec![Qualification::BoatPermitForType(BoatType::Dinghy)])
        .or(Qualification::SailingLicense(SbfSee))
        .or(Qualification::SailingLicense(Sks))
        .or(Qualification::SailingLicense(Sss))
        .or(Qualification::SailingLicense(Shs))
        .or(Qualification::SailingLicense(Br))
}

fn rule(
    area: SailingArea,
    min_members: u8,
    min_persons: u8,
    co_skipper: Option<Requirement>,
) -> CrewRule {
    CrewRule {
        area,
        min_members,
        min_persons,
        co_skipper_required: co_skipper.is_some(),
        co_skipper_requirement: co_skipper.unwrap_or_default(),
    }
}

fn released(mut boat: Boat) -> Boat {
    boat.releases = vec![
        Release {
            season: 2025,
            by: FRANK,
            at: at(2025, 4, 12, 15, 0),
        },
        Release {
            season: 2026,
            by: FRANK,
            at: at(2026, 4, 18, 15, 0),
        },
    ];
    boat
}

pub fn filou() -> Boat {
    let mut boat = Boat::new(FILOU, "Filou", BoatType::Dinghy);
    boat.length_m = Some(5.40);
    boat.beam_m = Some(1.80);
    boat.draft_m = Some(1.10);
    boat.sail_area_m2 = Some(15.0);
    boat.crew_rules = vec![rule(SailingArea::HomeWaters, 1, 1, None)];
    released(boat)
}

pub fn dipl_ing_ling() -> Boat {
    let mut boat = Boat::new(DIPL_ING_LING, "Dipl.Ing.ling", BoatType::Dinghy);
    boat.class = Some("Yngling".into());
    boat.length_m = Some(6.35);
    boat.beam_m = Some(1.73);
    boat.draft_m = Some(1.05);
    boat.sail_area_m2 = Some(14.0);
    boat.crew_rules = vec![rule(SailingArea::HomeWaters, 1, 1, None)];
    released(boat)
}

pub fn harald_blauzahn() -> Boat {
    let mut boat = Boat::new(HARALD_BLAUZAHN, "Harald Blauzahn", BoatType::Keelboat);
    boat.class = Some("Nordisches Folkeboot".into());
    boat.year_built = Some(1996);
    boat.length_m = Some(7.64);
    boat.beam_m = Some(2.20);
    boat.draft_m = Some(1.20);
    boat.sail_area_m2 = Some(24.0);
    boat.berths = Some(4);
    boat.skipper_requirement = Requirement::none().and(sks_or_higher());
    boat.crew_rules = vec![
        rule(SailingArea::HomeWaters, 2, 2, None),
        rule(
            SailingArea::Outside,
            2,
            2,
            Some(Requirement::none().and(sailing_skills())),
        ),
    ];
    released(boat)
}

/// Regeln für Dehler 31 und Matcher 37 (Segelordnung 1.2, 2.3).
fn cruiser_rules(mut boat: Boat) -> Boat {
    boat.skipper_requirement = Requirement::none()
        .and(sks_or_higher())
        .and(AnyOf::sailing_licenses(&[SailingLicenseType::SbfSee]))
        .and(AnyOf::radio_licenses(&[
            RadioLicenseType::Src,
            RadioLicenseType::Lrc,
        ]));
    boat.crew_rules = vec![
        rule(
            SailingArea::HomeWaters,
            2,
            2,
            Some(Requirement::none().and(sailing_skills())),
        ),
        rule(
            SailingArea::Outside,
            2,
            3,
            Some(
                Requirement::none()
                    .and(AnyOf(vec![Qualification::BoatPermit(HARALD_BLAUZAHN)]))
                    .and(AnyOf::sailing_licenses(&[SailingLicenseType::SbfSee])),
            ),
        ),
    ];
    released(boat)
}

pub fn schruvtrekker() -> Boat {
    let mut boat = Boat::new(SCHRUVTREKKER, "Schruvtrekker", BoatType::Keelboat);
    boat.class = Some("Dehler 31".into());
    boat.year_built = Some(1986);
    boat.length_m = Some(9.40);
    boat.draft_m = Some(1.50);
    boat.engine = Some("Einbaudiesel".into());
    cruiser_rules(boat)
}

pub fn trigon() -> Boat {
    let mut boat = Boat::new(TRIGON, "Trigon", BoatType::Keelboat);
    boat.class = Some("Matcher 37".into());
    boat.year_built = Some(1996);
    boat.length_m = Some(11.10);
    boat.draft_m = Some(1.95);
    boat.engine = Some("18 PS".into());
    cruiser_rules(boat)
}

pub fn boats() -> Vec<Boat> {
    vec![
        filou(),
        dipl_ing_ling(),
        harald_blauzahn(),
        schruvtrekker(),
        trigon(),
    ]
}

// --- Ämter ----------------------------------------------------------------

fn office(
    n: u128,
    role: Role,
    member: MemberId,
    start: Timestamp,
    end: Option<Timestamp>,
) -> Office {
    Office {
        id: OfficeId::from_uuid(Uuid::from_u128(0x0ff1_0000 + n)),
        role,
        member,
        start,
        end,
    }
}

/// Vorstand, Revisoren, Bootswarte und Abnehmer – einschließlich eines
/// beendeten Amtes als Historie.
pub fn offices() -> Vec<Office> {
    let mv2025 = at(2025, 4, 26, 19, 0);
    vec![
        office(
            0,
            Role::ViceChair,
            EVA,
            at(2004, 4, 24, 19, 0),
            Some(at(2024, 4, 20, 19, 0)),
        ),
        office(1, Role::Chair, EVA, at(2024, 4, 20, 19, 0), None),
        office(2, Role::ViceChair, ANNA, mv2025, None),
        office(3, Role::Treasurer, JONAS, at(2019, 4, 27, 19, 0), None),
        office(4, Role::Secretary, KARIN, at(2022, 4, 23, 19, 0), None),
        office(5, Role::HeadBoatwarden, FRANK, at(2020, 4, 25, 19, 0), None),
        office(6, Role::Auditor, LUKAS, mv2025, None),
        office(7, Role::Auditor, CLARA, mv2025, None),
        office(
            8,
            Role::Boatwarden(TRIGON),
            FRANK,
            at(2015, 4, 25, 19, 0),
            Some(mv2025),
        ),
        office(9, Role::Boatwarden(TRIGON), CLARA, mv2025, None),
        office(10, Role::Boatwarden(TRIGON), LUKAS, mv2025, None),
        office(
            11,
            Role::Boatwarden(SCHRUVTREKKER),
            ANNA,
            at(2023, 4, 22, 19, 0),
            None,
        ),
        office(
            12,
            Role::Boatwarden(HARALD_BLAUZAHN),
            BEN,
            at(2026, 4, 1, 0, 0),
            None,
        ),
        office(13, Role::Boatwarden(FILOU), DAVID, mv2025, None),
        office(14, Role::Boatwarden(DIPL_ING_LING), DAVID, mv2025, None),
        office(15, Role::Examiner(FILOU), DAVID, at(2025, 5, 1, 0, 0), None),
        office(
            16,
            Role::Examiner(HARALD_BLAUZAHN),
            FRANK,
            at(2020, 5, 1, 0, 0),
            None,
        ),
    ]
}

// --- Bootsabnahmen und Widerrufe ------------------------------------------

fn checkout(
    n: u128,
    member: MemberId,
    boat: BoatId,
    at: Timestamp,
    examiner: MemberId,
) -> BoatCheckout {
    BoatCheckout {
        id: CheckoutId::from_uuid(Uuid::from_u128(0xc4ec_0000 + n)),
        member,
        boat,
        at,
        result: CheckoutResult::Passed,
        examiner,
        remark: None,
    }
}

/// Bootsabnahmen seit 2005; vor 2020 hat Eva (damals 2. Vorsitzende)
/// abgenommen, danach Frank als Oberbootswart bzw. die bestellten Abnehmer.
pub fn boat_checkouts() -> Vec<BoatCheckout> {
    let mut ben_failed = checkout(5, BEN, HARALD_BLAUZAHN, at(2013, 5, 11, 15, 0), EVA);
    ben_failed.result = CheckoutResult::Failed;
    ben_failed.remark = Some("Anlegemanöver noch unsicher".into());
    vec![
        checkout(1, ANNA, FILOU, at(2015, 6, 13, 14, 0), EVA),
        checkout(2, ANNA, HARALD_BLAUZAHN, at(2016, 5, 21, 14, 0), EVA),
        checkout(3, ANNA, SCHRUVTREKKER, at(2018, 5, 26, 14, 0), EVA),
        checkout(4, ANNA, TRIGON, at(2021, 6, 19, 14, 0), FRANK),
        ben_failed,
        checkout(6, BEN, HARALD_BLAUZAHN, at(2013, 8, 17, 14, 0), EVA),
        checkout(7, BEN, TRIGON, at(2017, 6, 10, 14, 0), EVA),
        checkout(8, BEN, FILOU, at(2012, 6, 2, 14, 0), EVA),
        checkout(9, CLARA, FILOU, at(2023, 6, 3, 14, 0), FRANK),
        checkout(10, CLARA, HARALD_BLAUZAHN, at(2025, 6, 14, 14, 0), FRANK),
        checkout(11, CLARA, DIPL_ING_LING, at(2024, 5, 11, 14, 0), FRANK),
        checkout(12, DAVID, FILOU, at(2025, 5, 24, 14, 0), FRANK),
        checkout(13, FRANK, HARALD_BLAUZAHN, at(2005, 7, 9, 14, 0), EVA),
        checkout(14, FRANK, SCHRUVTREKKER, at(2006, 6, 17, 14, 0), EVA),
        checkout(15, FRANK, TRIGON, at(2007, 6, 16, 14, 0), EVA),
        checkout(16, EVA, HARALD_BLAUZAHN, at(2020, 6, 6, 14, 0), FRANK),
        checkout(17, EVA, TRIGON, at(2020, 6, 6, 16, 0), FRANK),
        checkout(18, KARIN, HARALD_BLAUZAHN, at(2019, 7, 6, 14, 0), EVA),
        checkout(19, JONAS, FILOU, at(2011, 5, 14, 14, 0), EVA),
        checkout(20, DAVID, DIPL_ING_LING, at(2025, 6, 21, 14, 0), FRANK),
    ]
}

pub fn revocations() -> Vec<Revocation> {
    vec![Revocation {
        id: RevocationId::from_uuid(Uuid::from_u128(0x2e70_0001)),
        member: BEN,
        boat: TRIGON,
        at: at(2025, 9, 6, 10, 0),
        reason: "Grundberührung ohne Meldung – erneute Abnahme erforderlich".into(),
        by: EVA,
    }]
}

// --- Arbeitstermine, Aufgaben und Arbeitsstunden ---------------------------

/// Einkranen am 18.04.2026 (ohne bestimmtes Boot).
pub const EINKRANEN: WorkEventId = WorkEventId::from_uuid(Uuid::from_u128(0xe7e7_0001));
/// Arbeitsdienst am Folkeboot im März 2026.
pub const FOLKEBOOT_DIENST: WorkEventId = WorkEventId::from_uuid(Uuid::from_u128(0xe7e7_0002));
/// Auskranen am 17.10.2026.
pub const AUSKRANEN: WorkEventId = WorkEventId::from_uuid(Uuid::from_u128(0xe7e7_0003));

pub const TRIGON_ANTIFOULING: TaskId = TaskId::from_uuid(Uuid::from_u128(0x7a5c_0001));
pub const FOCK_NAEHEN: TaskId = TaskId::from_uuid(Uuid::from_u128(0x7a5c_0002));
pub const VEREINSZEITUNG: TaskId = TaskId::from_uuid(Uuid::from_u128(0x7a5c_0003));
pub const WEBSEITE: TaskId = TaskId::from_uuid(Uuid::from_u128(0x7a5c_0004));
pub const SEGELTUCH: TaskId = TaskId::from_uuid(Uuid::from_u128(0x7a5c_0005));

struct Work {
    n: u128,
    member: MemberId,
    extent: WorkExtent,
    subject: WorkSubject,
    description: &'static str,
    reference: Option<WorkReference>,
    confirmed_by: Option<MemberId>,
}

impl Work {
    fn new(
        n: u128,
        member: MemberId,
        extent: WorkExtent,
        subject: WorkSubject,
        description: &'static str,
    ) -> Self {
        Self {
            n,
            member,
            extent,
            subject,
            description,
            reference: None,
            confirmed_by: None,
        }
    }

    fn at_event(mut self, event: WorkEventId) -> Self {
        self.reference = Some(WorkReference::WorkEvent(event));
        self
    }

    fn for_task(mut self, task: TaskId) -> Self {
        self.reference = Some(WorkReference::Task(task));
        self
    }

    fn confirmed_by(mut self, member: MemberId) -> Self {
        self.confirmed_by = Some(member);
        self
    }

    fn build(self) -> WorkEntry {
        let done = match &self.extent {
            WorkExtent::Amount { date, .. } => *date,
            WorkExtent::Period { end, .. } => *end,
        };
        WorkEntry {
            id: WorkEntryId::from_uuid(Uuid::from_u128(0x3e00_0000 + self.n)),
            member: self.member,
            extent: self.extent,
            subject: self.subject,
            description: Some(self.description.into()),
            reference: self.reference,
            confirmation: self.confirmed_by.map(|by| Confirmation {
                by,
                at: done + chrono::Duration::days(2),
            }),
        }
    }
}

fn hours(year: i32, month: u32, day: u32, hours: f64) -> WorkExtent {
    WorkExtent::Amount {
        date: at(year, month, day, 0, 0),
        hours,
    }
}

fn einkranen(start_hour: u32) -> WorkExtent {
    WorkExtent::Period {
        start: at(2026, 4, 18, start_hour, 0),
        end: at(2026, 4, 18, 15, 0),
    }
}

/// Arbeitseinsätze der Saison 2026 (und ein Einsatz über den Jahreswechsel).
///
/// Anrechenbare Stunden 2026: Anna 25, Ben 30 − 5 übertragen, Clara 20,5
/// (davon 3 h abgegolten, 6 h unbestätigt nicht gezählt), David 25 inkl.
/// 2 h von Clara, Eva 10 (reduziertes Soll), Frank 30, Karin 25, Lukas 12,
/// Malte 12 (reduziertes Soll); Jonas gilt seine 25 h finanziell ab.
pub fn work_entries() -> Vec<WorkEntry> {
    use WorkSubject::{Boat, Club};
    vec![
        // Anna
        Work::new(1, ANNA, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        Work::new(
            2,
            ANNA,
            hours(2026, 3, 7, 8.0),
            Boat(SCHRUVTREKKER),
            "Motor-Service Schruvtrekker",
        )
        .confirmed_by(FRANK),
        Work::new(
            3,
            ANNA,
            hours(2026, 3, 28, 5.0),
            Boat(TRIGON),
            "Polster der Trigon gereinigt",
        )
        .confirmed_by(CLARA),
        Work::new(
            4,
            ANNA,
            hours(2026, 5, 9, 6.0),
            Club,
            "Vereinsraum gestrichen",
        )
        .confirmed_by(EVA),
        // Ben
        Work::new(5, BEN, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        Work::new(
            6,
            BEN,
            hours(2026, 2, 21, 4.0),
            Boat(HARALD_BLAUZAHN),
            "Fock genäht",
        )
        .for_task(FOCK_NAEHEN)
        .confirmed_by(FRANK),
        Work::new(
            7,
            BEN,
            WorkExtent::Period {
                start: at(2026, 3, 14, 9, 0),
                end: at(2026, 3, 14, 17, 0),
            },
            Boat(HARALD_BLAUZAHN),
            "Arbeitsdienst Folkeboot",
        )
        .at_event(FOLKEBOOT_DIENST)
        .confirmed_by(FRANK),
        Work::new(
            8,
            BEN,
            hours(2026, 3, 21, 6.0),
            Boat(FILOU),
            "Schwert der Filou überholt",
        )
        .confirmed_by(DAVID),
        Work::new(9, BEN, hours(2026, 5, 16, 6.0), Club, "Steg repariert").confirmed_by(JONAS),
        // Clara
        Work::new(
            10,
            CLARA,
            hours(2026, 3, 28, 4.0),
            Boat(TRIGON),
            "Antifouling Trigon",
        )
        .for_task(TRIGON_ANTIFOULING)
        .confirmed_by(LUKAS),
        Work::new(
            11,
            CLARA,
            hours(2026, 4, 4, 2.5),
            Boat(TRIGON),
            "Rigg der Trigon kontrolliert",
        )
        .confirmed_by(LUKAS),
        Work::new(12, CLARA, einkranen(7), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        Work::new(
            13,
            CLARA,
            hours(2026, 5, 2, 6.0),
            Boat(FILOU),
            "Segel der Filou geflickt",
        ),
        // David
        Work::new(
            14,
            DAVID,
            hours(2026, 2, 10, 3.0),
            Club,
            "Webseite aktualisiert",
        )
        .for_task(WEBSEITE)
        .confirmed_by(KARIN),
        Work::new(15, DAVID, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        Work::new(
            16,
            DAVID,
            hours(2026, 3, 21, 8.0),
            Boat(FILOU),
            "Rumpf der Filou geschliffen",
        )
        .confirmed_by(FRANK),
        Work::new(
            17,
            DAVID,
            hours(2026, 3, 22, 6.0),
            Boat(DIPL_ING_LING),
            "Dipl.Ing.ling: Beschläge erneuert",
        )
        .confirmed_by(FRANK),
        // Eva
        Work::new(
            18,
            EVA,
            hours(2026, 1, 24, 4.0),
            Club,
            "Chronik für die Vereinszeitung",
        )
        .confirmed_by(KARIN),
        Work::new(19, EVA, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        // Frank
        Work::new(
            20,
            FRANK,
            hours(2026, 2, 28, 10.0),
            Boat(TRIGON),
            "Trigon: Winterlagerarbeiten",
        )
        .confirmed_by(CLARA),
        Work::new(21, FRANK, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(EVA),
        Work::new(
            22,
            FRANK,
            hours(2026, 3, 14, 8.0),
            Boat(HARALD_BLAUZAHN),
            "Arbeitsdienst Folkeboot",
        )
        .at_event(FOLKEBOOT_DIENST)
        .confirmed_by(EVA),
        Work::new(
            23,
            FRANK,
            hours(2026, 3, 7, 6.0),
            Boat(SCHRUVTREKKER),
            "Seeventile der Schruvtrekker getauscht",
        )
        .confirmed_by(ANNA),
        Work::new(
            24,
            FRANK,
            WorkExtent::Period {
                start: at(2025, 12, 31, 22, 0),
                end: at(2026, 1, 1, 1, 0),
            },
            Club,
            "Silvesterwache Bootshalle",
        )
        .confirmed_by(EVA),
        // Karin
        Work::new(
            25,
            KARIN,
            hours(2026, 2, 20, 7.0),
            Club,
            "Vereinszeitung: Redaktion",
        )
        .for_task(VEREINSZEITUNG)
        .confirmed_by(EVA),
        Work::new(
            26,
            KARIN,
            hours(2026, 3, 5, 5.0),
            Club,
            "Vereinszeitung: Layout und Druck",
        )
        .for_task(VEREINSZEITUNG)
        .confirmed_by(EVA),
        Work::new(27, KARIN, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        Work::new(
            28,
            KARIN,
            hours(2026, 3, 14, 7.0),
            Boat(HARALD_BLAUZAHN),
            "Lack am Folkeboot ausgebessert",
        )
        .at_event(FOLKEBOOT_DIENST)
        .confirmed_by(FRANK),
        // Lukas
        Work::new(
            29,
            LUKAS,
            hours(2026, 3, 28, 6.0),
            Boat(TRIGON),
            "Antifouling Trigon",
        )
        .for_task(TRIGON_ANTIFOULING)
        .confirmed_by(CLARA),
        Work::new(30, LUKAS, einkranen(9), Club, "Einkranen")
            .at_event(EINKRANEN)
            .confirmed_by(FRANK),
        // Malte (Eintritt im Mai)
        Work::new(
            31,
            MALTE,
            hours(2026, 6, 6, 6.0),
            Club,
            "Bootshalle aufgeräumt",
        )
        .confirmed_by(FRANK),
        Work::new(
            32,
            MALTE,
            hours(2026, 6, 13, 6.0),
            Boat(FILOU),
            "Filou: Persenning ausgebessert",
        )
        .confirmed_by(DAVID),
    ]
    .into_iter()
    .map(Work::build)
    .collect()
}

pub fn hour_transfers() -> Vec<HourTransfer> {
    vec![
        HourTransfer {
            id: HourTransferId::from_uuid(Uuid::from_u128(0x7f00_0001)),
            year: 2026,
            from: BEN,
            to: CLARA,
            hours: 5.0,
            description: Some("Ausgleich unter Partnern".into()),
        },
        HourTransfer {
            id: HourTransferId::from_uuid(Uuid::from_u128(0x7f00_0002)),
            year: 2026,
            from: CLARA,
            to: DAVID,
            hours: 2.0,
            description: Some("Ausgleich unter Partnern".into()),
        },
    ]
}

pub fn work_events() -> Vec<WorkEvent> {
    let mut einkranen = WorkEvent::new(
        "Boote kranen",
        at(2026, 4, 18, 9, 0),
        at(2026, 4, 18, 15, 0),
    );
    einkranen.id = EINKRANEN;
    einkranen.location = Some("Sporthafen Düsternbrook".into());
    einkranen.description = Some("Einkranen aller Kielboote, danach Grillen".into());
    einkranen.registrations = vec![ANNA, BEN, CLARA, DAVID, EVA, FRANK, KARIN, LUKAS];

    let mut folkeboot = WorkEvent::new(
        "Arbeitsdienst Folkeboot",
        at(2026, 3, 14, 9, 0),
        at(2026, 3, 14, 17, 0),
    );
    folkeboot.id = FOLKEBOOT_DIENST;
    folkeboot.location = Some("Bootshalle Schwentine".into());
    folkeboot.boat = Some(HARALD_BLAUZAHN);
    folkeboot.max_participants = Some(6);
    folkeboot.registrations = vec![BEN, FRANK, KARIN];

    let mut auskranen = WorkEvent::new(
        "Boote auskranen",
        at(2026, 10, 17, 9, 0),
        at(2026, 10, 17, 15, 0),
    );
    auskranen.id = AUSKRANEN;
    auskranen.location = Some("Sporthafen Düsternbrook".into());
    auskranen.registrations = vec![ANNA, FRANK, MALTE];

    vec![folkeboot, einkranen, auskranen]
}

pub fn tasks() -> Vec<Task> {
    let mut antifouling = Task::new("Trigon: Antifouling streichen");
    antifouling.id = TRIGON_ANTIFOULING;
    antifouling.boat = Some(TRIGON);
    antifouling.assignees = vec![CLARA, LUKAS];
    antifouling.estimated_hours = Some(10.0);
    antifouling.due_at = Some(at(2026, 4, 10, 0, 0));
    antifouling.set_status(TaskStatus::Done, at(2026, 3, 28, 18, 0));

    let mut fock = Task::new("Fock von Harald Blauzahn nähen");
    fock.id = FOCK_NAEHEN;
    fock.boat = Some(HARALD_BLAUZAHN);
    fock.assignees = vec![BEN];
    fock.estimated_hours = Some(4.0);
    fock.set_status(TaskStatus::Done, at(2026, 2, 21, 17, 0));

    let mut zeitung = Task::new("Vereinszeitung Frühjahrsausgabe");
    zeitung.id = VEREINSZEITUNG;
    zeitung.assignees = vec![KARIN];
    zeitung.estimated_hours = Some(12.0);
    zeitung.set_status(TaskStatus::Done, at(2026, 3, 5, 20, 0));

    let mut webseite = Task::new("Webseite: Törnberichte einpflegen");
    webseite.id = WEBSEITE;
    webseite.description = Some("Berichte der Saison 2025 mit Fotos".into());
    webseite.assignees = vec![DAVID];
    webseite.estimated_hours = Some(5.0);
    webseite.status = TaskStatus::InProgress;

    let mut segeltuch = Task::new("Segeltuch für die Fock bestellen");
    segeltuch.id = SEGELTUCH;
    segeltuch.description = Some("Wartet auf das **Angebot** der Segelmacherei.".into());
    segeltuch.boat = Some(HARALD_BLAUZAHN);
    segeltuch.assignees = vec![BEN];
    segeltuch.due_at = Some(at(2026, 11, 15, 0, 0));
    segeltuch.status = TaskStatus::Waiting;

    vec![antifouling, fock, zeitung, webseite, segeltuch]
}

// --- Reservierungen --------------------------------------------------------

/// David segelt allein die Filou.
pub const JOLLE_SAMSTAG: ReservationId = ReservationId::from_uuid(Uuid::from_u128(0x2e5e_0001));
/// Karin und Jonas auf dem Folkeboot im Heimrevier.
pub const FOLKEBOOT_NACHMITTAG: ReservationId =
    ReservationId::from_uuid(Uuid::from_u128(0x2e5e_0002));
/// Zwei Wochen Dänemark mit der Dehler: Crewwechsel und ein Gast.
pub const DEHLER_TOUR: ReservationId = ReservationId::from_uuid(Uuid::from_u128(0x2e5e_0003));
/// Wochenendtour mit der Trigon; Lukas fehlt die Segelerlaubnis.
pub const TRIGON_WOCHENENDE: ReservationId = ReservationId::from_uuid(Uuid::from_u128(0x2e5e_0004));

pub fn reservations() -> Vec<Reservation> {
    let mut jolle = Reservation::new(
        FILOU,
        at(2026, 6, 13, 14, 0),
        at(2026, 6, 13, 18, 0),
        SailingArea::HomeWaters,
        DAVID,
    );
    jolle.id = JOLLE_SAMSTAG;

    let mut folkeboot = Reservation::new(
        HARALD_BLAUZAHN,
        at(2026, 6, 20, 10, 0),
        at(2026, 6, 20, 17, 0),
        SailingArea::HomeWaters,
        KARIN,
    );
    folkeboot.id = FOLKEBOOT_NACHMITTAG;
    folkeboot.crew = vec![CrewSeat::new(JONAS)];

    let mut dehler = Reservation::new(
        SCHRUVTREKKER,
        at(2026, 7, 1, 10, 0),
        at(2026, 7, 14, 16, 0),
        SailingArea::Outside,
        ANNA,
    );
    dehler.id = DEHLER_TOUR;
    dehler.co_skipper = Some(KARIN);
    dehler.crew = vec![
        CrewSeat {
            member: BEN,
            from: None,
            until: Some(at(2026, 7, 7, 18, 0)),
        },
        CrewSeat {
            member: DAVID,
            from: Some(at(2026, 7, 7, 10, 0)),
            until: None,
        },
        CrewSeat::new(HANNES),
    ];
    dehler.remark = Some(
        "Kiel – Marstal – Svendborg – Kiel\nGast: Max Gast (0160 5554443), bis 3. Juli an Bord"
            .into(),
    );

    let mut trigon = Reservation::new(
        TRIGON,
        at(2026, 8, 14, 16, 0),
        at(2026, 8, 16, 18, 0),
        SailingArea::Outside,
        EVA,
    );
    trigon.id = TRIGON_WOCHENENDE;
    trigon.co_skipper = Some(FRANK);
    trigon.crew = vec![CrewSeat::new(LUKAS)];
    trigon.remark = Some("Gast: Ole Svensson (eingeladen von Frank)".into());

    vec![jolle, folkeboot, dehler, trigon]
}

// --- Saison, Jahresnachweise und Mitgliedskonten ---------------------------

pub fn seasons() -> Vec<Season> {
    let season = |year| Season {
        year,
        hours_quota: Some(25.0),
        compensation_rate: Some(Money::from_cents(1500)),
        fees: None,
    };
    vec![season(2025), season(2026)]
}

pub fn annual_records() -> Vec<AnnualRecord> {
    let reduced = |member, hours: f64, reason: &str| AnnualRecord {
        member,
        year: 2026,
        reduced_quota: Some(Reduction {
            hours,
            reason: reason.into(),
        }),
        compensation: None,
    };
    let compensated = |member, hours: f64, approved_at, reason: &str| AnnualRecord {
        member,
        year: 2026,
        reduced_quota: None,
        compensation: Some(Compensation {
            hours,
            amount: None,
            approved_at,
            reason: Some(reason.into()),
        }),
    };
    vec![
        reduced(EVA, 10.0, "Alter, Absprache mit dem Vorstand"),
        reduced(MALTE, 12.0, "Eintritt im Mai 2026"),
        compensated(CLARA, 3.0, at(2026, 6, 1, 0, 0), "Prüfungsphase"),
        compensated(
            JONAS,
            25.0,
            at(2026, 3, 1, 0, 0),
            "Auslandsaufenthalt bis Juli",
        ),
    ]
}

/// Einzahlungen der Saison 2026: Lukas hat noch nicht gezahlt, Anna hat 10 €
/// zu viel überwiesen, Claras Abgeltung und Davids Tourgebühr sind noch offen.
fn payments(member: MemberId) -> Vec<(Money, Timestamp, &'static str)> {
    let lastschrift = at(2026, 4, 1, 0, 0);
    let eur = |cents| Money::from_cents(cents);
    let beitrag = |cents| (eur(cents), lastschrift, "Lastschrift Jahresbeitrag 2026");
    match member {
        ANNA => vec![
            (eur(25000), lastschrift, "Überweisung Jahresbeitrag 2026"),
            (eur(14000), at(2026, 7, 20, 0, 0), "Tourgebühr Dänemark"),
        ],
        BEN => vec![
            beitrag(24000),
            (eur(7000), at(2026, 7, 18, 0, 0), "Tourgebühr Dänemark"),
        ],
        KARIN => vec![
            beitrag(24000),
            (eur(14000), at(2026, 7, 25, 0, 0), "Tourgebühr Dänemark"),
        ],
        FRANK => vec![
            beitrag(24000),
            (eur(3600), at(2026, 8, 20, 0, 0), "Tourgebühr Trigon"),
        ],
        HANNES => vec![(eur(10000), at(2026, 7, 20, 0, 0), "Tourgebühr Hannes")],
        EVA => vec![(eur(3600), at(2026, 8, 20, 0, 0), "Tourgebühr Trigon")],
        CLARA | DAVID => vec![beitrag(17000)],
        GRETA => vec![beitrag(6000)],
        INES => vec![(
            eur(10000),
            at(2026, 3, 4, 0, 0),
            "Überweisung Nutzungspauschale",
        )],
        JONAS => vec![
            (
                eur(37500),
                at(2026, 3, 5, 0, 0),
                "Überweisung Abgeltung Arbeitsstunden",
            ),
            (eur(24000), lastschrift, "Lastschrift Jahresbeitrag 2026"),
        ],
        MALTE => vec![(
            eur(14333),
            at(2026, 5, 20, 0, 0),
            "Überweisung Aufnahme und Beitrag",
        )],
        _ => Vec::new(),
    }
}

/// Mitglieder mit Forderungen 2026 (über die Beitrags- und Abgeltungsregeln
/// erzeugt), einer stornierten Forderung und den Einzahlungen.
fn with_accounts(mut club: ClubRecord) -> ClubRecord {
    let claims = club.annual_fee_claims(2026, at(2026, 3, 15, 10, 0));
    club.add_claims(claims);
    let claims = club.admission_fee_claims(2026, at(2026, 5, 16, 10, 0));
    club.add_claims(claims);
    let claims = club
        .compensation_claims(at(2026, 6, 2, 10, 0))
        .into_iter()
        .map(|(member, mut claim)| {
            claim.created_at = claim.due_at.expect("Abgeltung hat Fälligkeit");
            (member, claim)
        })
        .collect();
    club.add_claims(claims);
    for (reservation, created) in [
        (DEHLER_TOUR, at(2026, 7, 15, 10, 0)),
        (TRIGON_WOCHENENDE, at(2026, 8, 17, 10, 0)),
    ] {
        let r = club.reservation(reservation).expect("Reservierung").clone();
        let claims = club.tour_fee_claims(&r, created);
        club.add_claims(claims);
    }

    let ben = club.member_mut(BEN).expect("Ben");
    let mut deposit = Claim::new(
        ClaimKind::Other,
        Money::from_cents(2000),
        "Schlüsselpfand Bootshalle",
        at(2026, 2, 1, 10, 0),
        Some(at(2026, 2, 15, 0, 0)),
    );
    deposit.cancelled = Some(Cancellation {
        at: at(2026, 2, 14, 18, 0),
        reason: "Schlüssel zurückgegeben".into(),
    });
    ben.account.claims.insert(0, deposit);

    let (mut claim_no, mut payment_no) = (0u128, 0u128);
    for member in &mut club.members {
        for claim in &mut member.account.claims {
            claim_no += 1;
            claim.id = ClaimId::from_uuid(Uuid::from_u128(0xc1a1_0000 + claim_no));
        }
        for (amount, at, text) in payments(member.id) {
            payment_no += 1;
            member.account.payments.push(Payment {
                id: PaymentId::from_uuid(Uuid::from_u128(0x9a70_0000 + payment_no)),
                amount,
                at,
                text: Some(text.into()),
            });
        }
    }
    club
}

/// Die Beispielakte ohne Forderungen und Einzahlungen.
pub fn club_without_accounts() -> ClubRecord {
    let mut club = with_accounts(base_club());
    for member in &mut club.members {
        member.account = MemberAccount::default();
    }
    club
}

fn base_club() -> ClubRecord {
    ClubRecord {
        members: members(),
        boats: boats(),
        offices: offices(),
        boat_checkouts: boat_checkouts(),
        revocations: revocations(),
        work_entries: work_entries(),
        hour_transfers: hour_transfers(),
        work_events: work_events(),
        tasks: tasks(),
        seasons: seasons(),
        annual_records: annual_records(),
        reservations: reservations(),
    }
}

/// Die vollständige Beispielakte.
pub fn club() -> ClubRecord {
    with_accounts(base_club())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boat::SailingArea;

    #[test]
    fn example_club_can_be_built() {
        let club = club();
        assert_eq!(club.members.len(), 13);
        assert_eq!(club.boats.len(), 5);
        assert_eq!(
            at(2026, 5, 1, 10, 0).to_rfc3339(),
            "2026-05-01T10:00:00+02:00"
        );
    }

    #[test]
    fn example_club_is_valid() {
        assert_eq!(club().validate(), Ok(()));
    }

    #[test]
    fn crew_rules_follow_segelordnung() {
        for boat in boats() {
            assert!(
                boat.crew_rule(SailingArea::HomeWaters).is_some(),
                "{}",
                boat.name
            );
            let outside = boat.crew_rule(SailingArea::Outside).is_some();
            assert_eq!(
                outside,
                boat.boat_type == BoatType::Keelboat,
                "{}",
                boat.name
            );
        }
    }
}

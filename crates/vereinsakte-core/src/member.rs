//! Mitglieder: Stammdaten, Mitgliedsart, Mitgliedschaft, Scheine und Lizenzen.

use std::fmt;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::account::MemberAccount;
use super::ids::MemberId;
use super::time::{Timestamp, year_start};

/// Ein Mitglied der Segelgruppe. Die Kennung `id` ist die Mitgliedsnummer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id: MemberId,
    pub last_name: String,
    pub first_name: String,
    pub address: Address,
    #[serde(default, skip_serializing_if = "PhoneNumbers::is_empty")]
    pub phone: PhoneNumbers,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Adresse des Microsoft-Kontos für die Anmeldung (Anmeldekonto); getrennt
    /// von der Kontakt-E-Mail `email`, in der Vereinsakte eindeutig.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_email: Option<String>,
    /// Ist die Meldeadresse gültig? Ohne gültige Meldeadresse ruhen alle
    /// Bootserlaubnisse (Segelordnung 2.1).
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub address_valid: bool,
    pub membership_type: MembershipType,
    /// Bei Kindern von Mitgliedern: das Elternteil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_of: Option<MemberId>,
    /// Zustimmung des Vorstands zur Nutzung der Vereinseinrichtungen
    /// (nur für außerordentliche Mitglieder relevant, Satzung §4 Abs. 2.1).
    #[serde(default, skip_serializing_if = "is_false")]
    pub board_consent: bool,
    pub membership: Membership,
    /// Sperrvermerk: andere Mitglieder (außer dem Vorstand) sehen weder
    /// E-Mail noch Telefonnummern.
    #[serde(default, skip_serializing_if = "is_false")]
    pub contact_blocked: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sailing_licenses: Vec<SailingLicense>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub radio_licenses: Vec<RadioLicense>,
    #[serde(default, skip_serializing_if = "MemberAccount::is_empty")]
    pub account: MemberAccount,
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Member {
    /// Neues Mitglied mit den Pflichtangaben; alles Weitere ist leer bzw.
    /// auf den Standardwert gesetzt.
    pub fn new(
        id: MemberId,
        last_name: impl Into<String>,
        first_name: impl Into<String>,
        address: Address,
        membership_type: MembershipType,
        joined_at: Timestamp,
    ) -> Self {
        Self {
            id,
            last_name: last_name.into(),
            first_name: first_name.into(),
            address,
            phone: PhoneNumbers::default(),
            email: None,
            login_email: None,
            address_valid: true,
            membership_type,
            child_of: None,
            board_consent: false,
            membership: Membership::joined(joined_at),
            contact_blocked: false,
            sailing_licenses: Vec::new(),
            radio_licenses: Vec::new(),
            account: MemberAccount::default(),
        }
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_active_at(&self, t: Timestamp) -> bool {
        self.membership.is_active_at(t)
    }

    /// Ist das Mitglied an mindestens einem Zeitpunkt des Jahres aktiv?
    pub fn is_active_in(&self, year: i32) -> bool {
        self.membership
            .overlaps(year_start(year), year_start(year + 1))
    }

    pub fn is_provisional_at(&self, t: Timestamp) -> bool {
        self.is_active_at(t)
            && self
                .membership
                .provisional_until
                .is_some_and(|until| t < until)
    }

    /// Stimmberechtigt sind aktive, nicht vorläufig aufgenommene ordentliche,
    /// studentische und Ehrenmitglieder (Satzung §4, §5).
    pub fn has_voting_right_at(&self, t: Timestamp) -> bool {
        self.membership_type.has_voting_right()
            && self.is_active_at(t)
            && !self.is_provisional_at(t)
    }

    /// Austritt erklären: wirksam zum Ende des Geschäftsjahres (Satzung §7),
    /// also zum 1. Januar des Folgejahres, 00:00 Uhr Vereinszeit.
    pub fn declare_resignation(&mut self, declared_at: Timestamp) {
        self.membership.left_at = Some(year_start(declared_at.year() + 1));
    }

    pub fn has_sailing_license(&self, kind: &SailingLicenseType) -> bool {
        self.sailing_licenses.iter().any(|l| &l.kind == kind)
    }

    pub fn has_radio_license(&self, kind: &RadioLicenseType) -> bool {
        self.radio_licenses.iter().any(|l| &l.kind == kind)
    }

    /// Hinweis zur Meldeadresse, falls sie ungültig ist.
    pub fn address_issue(&self) -> Option<&'static str> {
        (!self.address_valid).then_some("Meldeadresse ungültig")
    }
}

/// Postanschrift. Die PLZ ist eine Zeichenkette, damit führende Nullen erhalten bleiben.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub postal_code: String,
    pub city: String,
}

impl Address {
    pub fn new(
        street: impl Into<String>,
        postal_code: impl Into<String>,
        city: impl Into<String>,
    ) -> Self {
        Self {
            street: street.into(),
            postal_code: postal_code.into(),
            city: city.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneNumbers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
}

impl PhoneNumbers {
    pub fn is_empty(&self) -> bool {
        self.landline.is_none() && self.business.is_none() && self.mobile.is_none()
    }
}

/// Mitgliedsart nach Satzung §3 und Beitragsordnung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipType {
    Regular,
    Student,
    Extraordinary,
    Honorary,
    ChildOfMember,
    Erasmus,
}

/// Tarifgruppe für Tourgebühren.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TariffGroup {
    Regular,
    Student,
    Extraordinary,
    Guest,
}

impl MembershipType {
    /// Besteht Beitragspflicht?
    pub fn pays_fee(self) -> bool {
        matches!(
            self,
            Self::Regular | Self::Student | Self::Extraordinary | Self::Erasmus
        )
    }

    /// Besteht Arbeitsstundenpflicht?
    pub fn owes_hours(self) -> bool {
        matches!(self, Self::Regular | Self::Student | Self::Honorary)
    }

    pub fn tariff_group(self) -> TariffGroup {
        match self {
            Self::Regular | Self::Honorary | Self::Erasmus => TariffGroup::Regular,
            Self::Student | Self::ChildOfMember => TariffGroup::Student,
            Self::Extraordinary => TariffGroup::Extraordinary,
        }
    }

    /// Stimmrecht nach Satzung §4 (ohne Berücksichtigung vorläufiger Aufnahme).
    pub fn has_voting_right(self) -> bool {
        matches!(self, Self::Regular | Self::Student | Self::Honorary)
    }
}

impl fmt::Display for MembershipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Regular => "ordentliches Mitglied",
            Self::Student => "studentisches Mitglied",
            Self::Extraordinary => "außerordentliches Mitglied",
            Self::Honorary => "Ehrenmitglied",
            Self::ChildOfMember => "Kind eines Mitglieds",
            Self::Erasmus => "Erasmus",
        })
    }
}

/// Verlauf der Mitgliedschaft. Alle Endzeitpunkte sind exklusiv.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub joined_at: Timestamp,
    /// Vorläufige Aufnahme (Satzung §5) bis zu diesem Zeitpunkt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provisional_until: Option<Timestamp>,
    /// Befristete Mitgliedschaft (z. B. Erasmus) bis zu diesem Zeitpunkt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limited_until: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expelled_at: Option<Timestamp>,
}

impl Membership {
    pub fn joined(joined_at: Timestamp) -> Self {
        Self {
            joined_at,
            provisional_until: None,
            limited_until: None,
            left_at: None,
            expelled_at: None,
        }
    }

    /// Frühester Endzeitpunkt aus Befristung, Austritt und Ausschluss.
    pub fn end(&self) -> Option<Timestamp> {
        [self.limited_until, self.left_at, self.expelled_at]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn is_active_at(&self, t: Timestamp) -> bool {
        self.joined_at <= t && self.end().is_none_or(|end| t < end)
    }

    /// Überschneidet sich die Mitgliedschaft mit `[start, end)`?
    pub fn overlaps(&self, start: Timestamp, end: Timestamp) -> bool {
        self.joined_at < end && self.end().is_none_or(|e| start < e)
    }
}

/// Art eines Segelscheins; im JSON eine einfache Zeichenkette.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum SailingLicenseType {
    SbfSee,
    SbfBinnen,
    Sks,
    Sss,
    Shs,
    /// Älterer DSV-Schein, weiterhin anerkannt.
    Br,
    Other(String),
}

impl From<&str> for SailingLicenseType {
    fn from(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "sbf see" => Self::SbfSee,
            "sbf binnen" => Self::SbfBinnen,
            "sks" => Self::Sks,
            "sss" => Self::Sss,
            "shs" => Self::Shs,
            "br" => Self::Br,
            _ => Self::Other(value.to_string()),
        }
    }
}

impl From<String> for SailingLicenseType {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<SailingLicenseType> for String {
    fn from(value: SailingLicenseType) -> Self {
        value.to_string()
    }
}

impl fmt::Display for SailingLicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SbfSee => "SBF See",
            Self::SbfBinnen => "SBF Binnen",
            Self::Sks => "SKS",
            Self::Sss => "SSS",
            Self::Shs => "SHS",
            Self::Br => "BR",
            Self::Other(name) => name,
        })
    }
}

/// Art einer Funklizenz; im JSON eine einfache Zeichenkette.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
pub enum RadioLicenseType {
    Src,
    Lrc,
    Ubi,
    Roc,
    Other(String),
}

impl From<&str> for RadioLicenseType {
    fn from(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "src" => Self::Src,
            "lrc" => Self::Lrc,
            "ubi" => Self::Ubi,
            "roc" => Self::Roc,
            _ => Self::Other(value.to_string()),
        }
    }
}

impl From<String> for RadioLicenseType {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<RadioLicenseType> for String {
    fn from(value: RadioLicenseType) -> Self {
        value.to_string()
    }
}

impl fmt::Display for RadioLicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Src => "SRC",
            Self::Lrc => "LRC",
            Self::Ubi => "UBI",
            Self::Roc => "ROC",
            Self::Other(name) => name,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SailingLicense {
    pub kind: SailingLicenseType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquired_at: Option<Timestamp>,
}

impl SailingLicense {
    pub fn new(kind: impl Into<SailingLicenseType>) -> Self {
        Self {
            kind: kind.into(),
            acquired_at: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioLicense {
    pub kind: RadioLicenseType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquired_at: Option<Timestamp>,
}

impl RadioLicense {
    pub fn new(kind: impl Into<RadioLicenseType>) -> Self {
        Self {
            kind: kind.into(),
            acquired_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::example::{self, at};

    #[test]
    fn license_types_are_plain_strings() {
        assert_eq!(
            serde_json::to_string(&SailingLicenseType::Sks).unwrap(),
            "\"SKS\""
        );
        assert_eq!(
            serde_json::to_string(&SailingLicenseType::SbfSee).unwrap(),
            "\"SBF See\""
        );
        let read: SailingLicenseType = serde_json::from_str("\"sbf see\"").unwrap();
        assert_eq!(read, SailingLicenseType::SbfSee);
        assert_eq!(serde_json::to_string(&read).unwrap(), "\"SBF See\"");
        assert_eq!(RadioLicenseType::from(" SRC "), RadioLicenseType::Src);
    }

    #[test]
    fn free_license_names_survive_round_trip() {
        let kind = SailingLicenseType::from("Jüngstenschein");
        assert_eq!(kind, SailingLicenseType::Other("Jüngstenschein".into()));
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"Jüngstenschein\"");
        assert_eq!(
            serde_json::from_str::<SailingLicenseType>(&json).unwrap(),
            kind
        );
    }

    #[test]
    fn other_with_known_name_is_normalized() {
        let json = serde_json::to_string(&SailingLicenseType::Other("SKS".into())).unwrap();
        assert_eq!(
            serde_json::from_str::<SailingLicenseType>(&json).unwrap(),
            SailingLicenseType::Sks
        );
    }

    #[test]
    fn member_keeps_number_and_postal_code() {
        let mut anna = example::anna();
        anna.address = Address::new("Prager Straße 12", "01067", "Dresden");
        let value = serde_json::to_value(&anna).unwrap();
        assert_eq!(value["id"], json!(101));
        assert_eq!(value["address"]["postal_code"], json!("01067"));
        let back: Member = serde_json::from_value(value).unwrap();
        assert_eq!(back, anna);

        let clara = serde_json::to_value(example::clara()).unwrap();
        assert!(clara["phone"].get("landline").is_none());
        assert!(
            serde_json::to_value(example::ben())
                .unwrap()
                .get("phone")
                .is_none()
        );
    }

    #[test]
    fn missing_last_name_is_rejected() {
        let mut value = serde_json::to_value(example::anna()).unwrap();
        value.as_object_mut().unwrap().remove("last_name");
        assert!(serde_json::from_value::<Member>(value).is_err());
    }

    #[test]
    fn optional_fields_default_on_read() {
        let value = json!({
            "id": 150,
            "last_name": "Kühl",
            "first_name": "Ole",
            "address": {"street": "Knooper Weg 3", "postal_code": "24103", "city": "Kiel"},
            "phone": {"mobile": "0170 1234567"},
            "membership_type": "regular",
            "membership": {"joined_at": "2024-03-01T00:00:00+01:00"}
        });
        let member: Member = serde_json::from_value(value).unwrap();
        assert!(member.address_valid);
        assert!(!member.board_consent);
        assert_eq!(member.phone.landline, None);
        assert_eq!(member.phone.mobile.as_deref(), Some("0170 1234567"));
        assert!(member.account.claims.is_empty());
    }

    #[test]
    fn duties_per_membership_type() {
        use MembershipType::*;
        let table = [
            (Regular, true, true, TariffGroup::Regular, true),
            (Student, true, true, TariffGroup::Student, true),
            (
                Extraordinary,
                true,
                false,
                TariffGroup::Extraordinary,
                false,
            ),
            (Honorary, false, true, TariffGroup::Regular, true),
            (ChildOfMember, false, false, TariffGroup::Student, false),
            (Erasmus, true, false, TariffGroup::Regular, false),
        ];
        for (kind, fee, hours, tariff, vote) in table {
            assert_eq!(kind.pays_fee(), fee, "{kind}");
            assert_eq!(kind.owes_hours(), hours, "{kind}");
            assert_eq!(kind.tariff_group(), tariff, "{kind}");
            assert_eq!(kind.has_voting_right(), vote, "{kind}");
        }
    }

    #[test]
    fn child_refers_to_parent() {
        let hannes = example::hannes();
        assert_eq!(hannes.membership_type, MembershipType::ChildOfMember);
        assert_eq!(hannes.child_of, Some(example::ANNA));
    }

    #[test]
    fn resignation_takes_effect_at_year_end() {
        let mut anna = example::anna();
        anna.declare_resignation(at(2026, 5, 10, 18, 0));
        assert_eq!(
            anna.membership.left_at.unwrap().to_rfc3339(),
            "2027-01-01T00:00:00+01:00"
        );
        assert!(anna.is_active_at(at(2026, 12, 31, 23, 59)));
        assert!(!anna.is_active_at(at(2027, 1, 1, 0, 0)));
    }

    #[test]
    fn erasmus_membership_is_limited() {
        let ines = example::ines();
        assert!(ines.is_active_at(at(2026, 6, 1, 12, 0)));
        assert!(!ines.is_active_at(at(2026, 11, 15, 12, 0)));
        assert!(!ines.is_active_in(2027));
    }

    #[test]
    fn expelled_member_is_inactive() {
        let mut ben = example::ben();
        ben.membership.expelled_at = Some(at(2026, 6, 1, 0, 0));
        assert!(ben.is_active_at(at(2026, 5, 31, 12, 0)));
        assert!(!ben.is_active_at(at(2026, 6, 2, 12, 0)));
        assert!(ben.is_active_in(2026));
    }

    #[test]
    fn provisional_member_is_active_but_marked() {
        let malte = example::malte();
        let t = at(2026, 6, 1, 12, 0);
        assert!(malte.is_active_at(t));
        assert!(malte.is_provisional_at(t));
        assert!(!malte.has_voting_right_at(t));
        assert!(!malte.is_provisional_at(at(2027, 5, 1, 12, 0)));
    }

    #[test]
    fn licenses_can_be_queried() {
        let clara = example::clara();
        assert!(clara.has_sailing_license(&SailingLicenseType::SbfSee));
        assert!(clara.has_sailing_license(&SailingLicenseType::Sks));
        assert!(!clara.has_sailing_license(&SailingLicenseType::Sss));
        assert!(example::anna().has_radio_license(&RadioLicenseType::Src));
    }

    #[test]
    fn login_account_is_separate_from_contact_email() {
        let value = serde_json::to_value(example::anna()).unwrap();
        assert_eq!(value["email"], json!("anna.petersen@example.org"));
        assert_eq!(value["login_email"], json!("anna.petersen@sgfh.example"));

        let hannes = serde_json::to_value(example::hannes()).unwrap();
        assert!(hannes.get("login_email").is_none());
        let back: Member = serde_json::from_value(hannes).unwrap();
        assert_eq!(back.login_email, None);
    }

    #[test]
    fn contact_block_defaults_to_off_and_is_only_written_when_set() {
        let anna = example::anna();
        assert!(!anna.contact_blocked);
        let value = serde_json::to_value(&anna).unwrap();
        assert!(value.get("contact_blocked").is_none());
        // Alte Daten ohne das Merkmal
        let back: Member = serde_json::from_value(value).unwrap();
        assert!(!back.contact_blocked);
        let mut blocked = anna;
        blocked.contact_blocked = true;
        let value = serde_json::to_value(&blocked).unwrap();
        assert_eq!(value["contact_blocked"], json!(true));
        let back: Member = serde_json::from_value(value).unwrap();
        assert!(back.contact_blocked);
    }

    #[test]
    fn invalid_address_is_reported() {
        let mut anna = example::anna();
        assert_eq!(anna.address_issue(), None);
        anna.address_valid = false;
        assert_eq!(anna.address_issue(), Some("Meldeadresse ungültig"));
        let value = serde_json::to_value(&anna).unwrap();
        assert_eq!(value["address_valid"], json!(false));
    }
}

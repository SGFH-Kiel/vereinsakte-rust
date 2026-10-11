//! Mitglieder ansehen und verwalten: Mitgliedsseite, Kontaktdaten, Aufnahme,
//! Stammdaten, Mitgliedschaftsverlauf sowie Segelscheine und Funklizenzen.

use crate::contact::ContactInfo;
use chrono::NaiveDate;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::member::{
    Address, Member, MembershipType, PhoneNumbers, RadioLicense, SailingLicense,
};
use vereinsakte_core::office::Offices;
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::club_time;

use chrono::Datelike;

use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::my_data::MyDataView;
use crate::ports::Version;
use crate::scope::{MemberScope, Scope};
use crate::service::{ClubService, CurrentUser, office_labels};

/// Mitgliedsseite: öffentliche Angaben für alle, Einzelheiten nur für Berechtigte.
#[derive(Clone, Debug, PartialEq)]
pub struct MemberPage {
    pub id: u32,
    pub name: String,
    pub membership_type: String,
    pub offices: Vec<String>,
    pub joined: String,
    pub left: Option<String>,
    pub active: bool,
    pub member_number: u32,
    /// E-Mail und Telefon für alle aktiven Mitglieder, außer bei Sperrvermerk.
    pub contact: ContactInfo,
    /// Anschrift, Segelerlaubnis, Stunden und Konto – nur mit `ReadMember`.
    pub details: Option<MyDataView>,
    pub is_self: bool,
    pub can_edit_profile: bool,
    pub can_manage: bool,
}

/// Eingaben der Kontaktdaten.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContactInput {
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub landline: String,
    pub business: String,
    pub mobile: String,
    pub email: String,
}

/// Eingaben der Stammdaten (Vorstand).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberInput {
    /// Leer bei „nächste freie Nummer“ (nur beim Aufnehmen).
    pub number: Option<u32>,
    pub first_name: String,
    pub last_name: String,
    pub contact: ContactInput,
    pub login_email: String,
    pub membership_type: MembershipType,
    pub board_consent: bool,
    pub address_valid: bool,
    pub child_of: Option<u32>,
    /// Eintrittsdatum (nur beim Aufnehmen).
    pub joined: Option<NaiveDate>,
}

/// Vorbelegung eines Bearbeitungsformulars samt Version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberForm {
    pub id: u32,
    pub version: i64,
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub contact: ContactInput,
    pub login_email: String,
    pub membership_type: MembershipType,
    pub board_consent: bool,
    pub address_valid: bool,
    pub child_of: Option<u32>,
    pub licenses: Vec<LicenseLine>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenseKind {
    Sailing,
    Radio,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicenseLine {
    pub kind: LicenseKind,
    pub index: usize,
    pub name: String,
    pub acquired: Option<String>,
}

/// Mitgliedsarten für Auswahllisten: (Schlüssel, Bezeichnung).
pub fn membership_types() -> Vec<(&'static str, String)> {
    ALL_TYPES
        .iter()
        .map(|(key, t)| (*key, t.to_string()))
        .collect()
}

const ALL_TYPES: [(&str, MembershipType); 6] = [
    ("regular", MembershipType::Regular),
    ("student", MembershipType::Student),
    ("extraordinary", MembershipType::Extraordinary),
    ("honorary", MembershipType::Honorary),
    ("child_of_member", MembershipType::ChildOfMember),
    ("erasmus", MembershipType::Erasmus),
];

/// Schlüssel einer Mitgliedsart (siehe [`membership_types`]).
pub fn parse_membership_type(key: &str) -> Option<MembershipType> {
    ALL_TYPES.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// Schlüssel zu einer Mitgliedsart.
pub fn membership_type_key(t: MembershipType) -> &'static str {
    ALL_TYPES
        .iter()
        .find(|(_, x)| *x == t)
        .map_or("regular", |(k, _)| k)
}

/// Symbol der Mitgliedsart für die Oberfläche.
pub fn membership_type_icon(t: MembershipType) -> &'static str {
    match t {
        MembershipType::Regular => "user",
        MembershipType::Student => "cap",
        MembershipType::Extraordinary => "user-plus",
        MembershipType::Honorary => "star",
        MembershipType::ChildOfMember => "smile",
        MembershipType::Erasmus => "globe",
    }
}

fn trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn rejected(field: &str, message: &str) -> FieldError {
    FieldError::new(field, message)
}

/// Prüft Pflichtfelder der Kontaktdaten.
fn check_contact(contact: &ContactInput, errors: &mut Vec<FieldError>) {
    for (field, value, label) in [
        ("street", &contact.street, "die Straße"),
        ("postal_code", &contact.postal_code, "die Postleitzahl"),
        ("city", &contact.city, "den Ort"),
    ] {
        if value.trim().is_empty() {
            errors.push(rejected(field, &format!("Bitte {label} angeben.")));
        }
    }
    if let Some(email) = trimmed(&contact.email)
        && (!email.contains('@') || email.contains(' '))
    {
        errors.push(rejected(
            "email",
            "Bitte eine gültige E-Mail-Adresse angeben.",
        ));
    }
}

/// Wendet die Kontaktdaten an und nennt die geänderten Felder.
fn apply_contact(member: &mut Member, c: &ContactInput) -> Vec<&'static str> {
    let mut changed = vec![];
    let address = Address::new(c.street.trim(), c.postal_code.trim(), c.city.trim());
    if member.address != address {
        member.address = address;
        changed.push("Anschrift");
    }
    let phone = PhoneNumbers {
        landline: trimmed(&c.landline),
        business: trimmed(&c.business),
        mobile: trimmed(&c.mobile),
    };
    if member.phone != phone {
        member.phone = phone;
        changed.push("Telefon");
    }
    let email = trimmed(&c.email);
    if member.email != email {
        member.email = email;
        changed.push("E-Mail");
    }
    changed
}

fn midnight(date: NaiveDate) -> vereinsakte_core::time::Timestamp {
    club_time(date.year(), date.month(), date.day(), 0, 0)
}

impl ClubService {
    async fn load_member(&self, id: MemberId) -> Result<crate::changes::Scoped, AppError> {
        let scoped = self
            .load(&Scope {
                members: MemberScope::Some(vec![id]),
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(id).is_none() {
            return Err(AppError::NotFound(format!("Mitglied {id}")));
        }
        Ok(scoped)
    }

    /// Mitgliedsseite mit Sichtbarkeit nach Berechtigung.
    pub async fn member_page(
        &self,
        user: &CurrentUser,
        id: MemberId,
    ) -> Result<MemberPage, AppError> {
        let principal = self.principal(user).await?;
        let member = self
            .store
            .member(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {id}")))?
            .value;
        let now = principal.now;
        let active = member.is_active_at(now);
        let may_read = principal.can(&Capability::ReadMember(id));
        // Ehemalige Mitglieder sieht nur, wer ihre Daten lesen darf.
        if !active && !may_read {
            return Err(AppError::NotFound(format!("Mitglied {id}")));
        }
        let offices = self.store.active_offices(now).await?;
        let boats = self.store.boats().await?;
        let labels = office_labels(&Offices::new(&offices), &boats, id, now);
        let contact = ContactInfo::of(
            &principal,
            id,
            member.contact_blocked,
            member.email.as_ref(),
            &member.phone,
        );
        let details = if may_read {
            let mut data = self.member_data(id).await?;
            if contact == ContactInfo::Blocked {
                // Auch Revisoren lesen gesperrte Kontaktdaten nicht.
                data.email = None;
                data.phones.clear();
            }
            Some(data)
        } else {
            None
        };
        let ended = member.membership.left_at.or(member.membership.expelled_at);
        Ok(MemberPage {
            id: id.get(),
            name: member.full_name(),
            membership_type: member.membership_type.to_string(),
            offices: labels,
            joined: format::date(member.membership.joined_at),
            left: ended.map(format::date),
            active,
            member_number: id.get(),
            contact,
            details,
            is_self: principal.member == id,
            can_edit_profile: principal.can(&Capability::EditProfile(id)),
            can_manage: principal.can(&Capability::ManageMembers),
        })
    }

    /// Vorbelegung des Bearbeitungsformulars (Kontaktdaten, Stammdaten, Scheine).
    pub async fn member_form(
        &self,
        user: &CurrentUser,
        id: MemberId,
    ) -> Result<MemberForm, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditProfile(id))?;
        let versioned = self
            .store
            .member(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {id}")))?;
        let m = versioned.value;
        let mut licenses = vec![];
        for (index, l) in m.sailing_licenses.iter().enumerate() {
            licenses.push(LicenseLine {
                kind: LicenseKind::Sailing,
                index,
                name: l.kind.to_string(),
                acquired: l.acquired_at.map(format::date),
            });
        }
        for (index, l) in m.radio_licenses.iter().enumerate() {
            licenses.push(LicenseLine {
                kind: LicenseKind::Radio,
                index,
                name: l.kind.to_string(),
                acquired: l.acquired_at.map(format::date),
            });
        }
        Ok(MemberForm {
            id: id.get(),
            version: versioned.version.0,
            name: m.full_name(),
            first_name: m.first_name.clone(),
            last_name: m.last_name.clone(),
            contact: ContactInput {
                street: m.address.street.clone(),
                postal_code: m.address.postal_code.clone(),
                city: m.address.city.clone(),
                landline: m.phone.landline.clone().unwrap_or_default(),
                business: m.phone.business.clone().unwrap_or_default(),
                mobile: m.phone.mobile.clone().unwrap_or_default(),
                email: m.email.clone().unwrap_or_default(),
            },
            login_email: m.login_email.clone().unwrap_or_default(),
            membership_type: m.membership_type,
            board_consent: m.board_consent,
            address_valid: m.address_valid,
            child_of: m.child_of.map(MemberId::get),
            licenses,
        })
    }

    /// Eigene Kontaktdaten (oder, als Vorstand, die eines anderen) ändern.
    pub async fn update_contact(
        &self,
        user: &CurrentUser,
        id: MemberId,
        version: i64,
        input: &ContactInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditProfile(id))?;
        let mut errors = vec![];
        check_contact(input, &mut errors);
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let old = self.load_member(id).await?;
        if old.version(crate::changes::RecordKind::Member, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        let changed = apply_contact(member, input);
        if changed.is_empty() {
            return Ok(());
        }
        let entry = audit(
            &principal,
            "member.contact",
            "member",
            id,
            Some(id),
            format!("Kontaktdaten geändert: {}", changed.join(", ")),
        );
        self.save(&old, &new, vec![entry]).await
    }

    /// Sperrvermerk für die eigenen Kontaktdaten setzen oder entfernen. Nur das
    /// Mitglied selbst darf das; E-Mail und Telefon bleiben für den Vorstand sichtbar.
    pub async fn set_contact_blocked(
        &self,
        user: &CurrentUser,
        id: MemberId,
        blocked: bool,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        if principal.member != id {
            return Err(AppError::Forbidden);
        }
        let old = self.load_member(id).await?;
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        member.contact_notice_seen = true;
        if member.contact_blocked == blocked {
            return self.save(&old, &new, vec![]).await;
        }
        member.contact_blocked = blocked;
        let entry = audit(
            &principal,
            "member.contact_block",
            "member",
            id,
            Some(id),
            "Sperrvermerk geändert".to_string(),
        );
        self.save(&old, &new, vec![entry]).await
    }

    /// Den einmaligen Hinweis zur Sichtbarkeit der Kontaktdaten bestätigen.
    pub async fn dismiss_contact_notice(&self, user: &CurrentUser) -> Result<(), AppError> {
        let id = user.member;
        let old = self.load_member(id).await?;
        if old.record.member(id).is_some_and(|m| m.contact_notice_seen) {
            return Ok(());
        }
        let mut new = old.record.clone();
        new.member_mut(id).expect("geladen").contact_notice_seen = true;
        self.save(&old, &new, vec![]).await
    }

    /// Mitglied aufnehmen (Vorstand).
    pub async fn create_member(
        &self,
        user: &CurrentUser,
        input: &MemberInput,
    ) -> Result<MemberId, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageMembers)?;
        let mut errors = vec![];
        check_contact(&input.contact, &mut errors);
        check_names(input, &mut errors);
        let joined = input.joined.unwrap_or_else(|| principal.now.date_naive());
        let number = match input.number {
            Some(n) => n,
            None => self.store.next_member_number().await?,
        };
        let id = MemberId::new(number);
        if id.is_none() {
            errors.push(rejected(
                "number",
                "Bitte eine Mitgliedsnummer größer 0 angeben.",
            ));
        }
        if let Some(id) = id
            && self.store.member(id).await?.is_some()
        {
            errors.push(rejected(
                "number",
                "Diese Mitgliedsnummer ist bereits vergeben.",
            ));
        }
        self.check_login(input, None, &mut errors).await?;
        let Some(id) = id.filter(|_| errors.is_empty()) else {
            return Err(AppError::Rejected(errors));
        };
        let mut member = Member::new(
            id,
            input.last_name.trim(),
            input.first_name.trim(),
            Address::new(
                input.contact.street.trim(),
                input.contact.postal_code.trim(),
                input.contact.city.trim(),
            ),
            input.membership_type,
            midnight(joined),
        );
        apply_contact(&mut member, &input.contact);
        member.login_email = trimmed(&input.login_email);
        member.board_consent = input.board_consent;
        member.address_valid = input.address_valid;
        member.child_of = input.child_of.and_then(MemberId::new);
        let entry = audit(
            &principal,
            "member.create",
            "member",
            id,
            Some(id),
            format!("Mitglied {} aufgenommen", member.full_name()),
        );
        self.store
            .commit(
                vec![crate::changes::Change::create(
                    crate::changes::Record::Member(member),
                )],
                vec![entry],
            )
            .await
            .map_err(login_conflict)?;
        Ok(id)
    }

    async fn check_login(
        &self,
        input: &MemberInput,
        own: Option<MemberId>,
        errors: &mut Vec<FieldError>,
    ) -> Result<(), AppError> {
        if let Some(login) = trimmed(&input.login_email) {
            if !login.contains('@') {
                errors.push(rejected(
                    "login_email",
                    "Bitte eine gültige Adresse angeben.",
                ));
            } else if let Some(other) = self.store.member_by_login_email(&login).await?
                && Some(other.id) != own
            {
                errors.push(rejected(
                    "login_email",
                    "Dieses Anmeldekonto gehört bereits einem anderen Mitglied.",
                ));
            }
        }
        Ok(())
    }

    /// Stammdaten, Mitgliedsart und Anmeldekonto ändern (Vorstand).
    pub async fn update_member(
        &self,
        user: &CurrentUser,
        id: MemberId,
        version: i64,
        input: &MemberInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageMembers)?;
        let mut errors = vec![];
        check_contact(&input.contact, &mut errors);
        check_names(input, &mut errors);
        self.check_login(input, Some(id), &mut errors).await?;
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let old = self.load_member(id).await?;
        if old.version(crate::changes::RecordKind::Member, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        let mut changed = apply_contact(member, &input.contact);
        let mut set = |name: &'static str, differs: bool| {
            if differs {
                changed.push(name);
            }
        };
        set(
            "Name",
            member.first_name != input.first_name.trim()
                || member.last_name != input.last_name.trim(),
        );
        member.first_name = input.first_name.trim().to_string();
        member.last_name = input.last_name.trim().to_string();
        let login = trimmed(&input.login_email);
        set("Anmeldekonto", member.login_email != login);
        member.login_email = login;
        set(
            "Mitgliedsart",
            member.membership_type != input.membership_type,
        );
        member.membership_type = input.membership_type;
        set(
            "Vorstandszustimmung",
            member.board_consent != input.board_consent,
        );
        member.board_consent = input.board_consent;
        set("Meldeadresse", member.address_valid != input.address_valid);
        member.address_valid = input.address_valid;
        let child_of = input.child_of.and_then(MemberId::new);
        set("Elternmitglied", member.child_of != child_of);
        member.child_of = child_of;
        if changed.is_empty() {
            return Ok(());
        }
        let entry = audit(
            &principal,
            "member.edit",
            "member",
            id,
            Some(id),
            format!("Mitglied geändert: {}", changed.join(", ")),
        );
        self.save(&old, &new, vec![entry])
            .await
            .map_err(login_conflict)
    }

    /// Austritt oder Ausschluss zum Datum erfassen (Vorstand).
    pub async fn end_membership(
        &self,
        user: &CurrentUser,
        id: MemberId,
        version: i64,
        on: NaiveDate,
        expelled: bool,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageMembers)?;
        let old = self.load_member(id).await?;
        if old.version(crate::changes::RecordKind::Member, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        let at = midnight(on);
        if at < member.membership.joined_at {
            return Err(AppError::rejected(
                "date",
                "Das Ende liegt vor dem Eintritt.",
            ));
        }
        if expelled {
            member.membership.expelled_at = Some(at);
        } else {
            member.membership.left_at = Some(at);
        }
        let entry = audit(
            &principal,
            if expelled {
                "member.expel"
            } else {
                "member.leave"
            },
            "member",
            id,
            Some(id),
            if expelled {
                "Ausschluss erfasst"
            } else {
                "Austritt erfasst"
            },
        );
        self.save(&old, &new, vec![entry]).await
    }

    /// Segelschein oder Funklizenz erfassen (eigene oder, als Vorstand, fremde).
    pub async fn add_license(
        &self,
        user: &CurrentUser,
        id: MemberId,
        kind: LicenseKind,
        name: &str,
        acquired: Option<NaiveDate>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditProfile(id))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::rejected("name", "Bitte die Bezeichnung angeben."));
        }
        let old = self.load_member(id).await?;
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        let at = acquired.map(midnight);
        match kind {
            LicenseKind::Sailing => {
                let mut license = SailingLicense::new(name);
                license.acquired_at = at;
                if member.sailing_licenses.contains(&license) {
                    return Err(AppError::rejected("name", "Diesen Schein gibt es bereits."));
                }
                member.sailing_licenses.push(license);
            }
            LicenseKind::Radio => {
                let mut license = RadioLicense::new(name);
                license.acquired_at = at;
                if member.radio_licenses.contains(&license) {
                    return Err(AppError::rejected("name", "Diese Lizenz gibt es bereits."));
                }
                member.radio_licenses.push(license);
            }
        }
        let entry = audit(
            &principal,
            "member.license.add",
            "member",
            id,
            Some(id),
            match kind {
                LicenseKind::Sailing => "Segelschein hinzugefügt",
                LicenseKind::Radio => "Funklizenz hinzugefügt",
            },
        );
        self.save(&old, &new, vec![entry]).await
    }

    /// Segelschein oder Funklizenz entfernen.
    pub async fn remove_license(
        &self,
        user: &CurrentUser,
        id: MemberId,
        kind: LicenseKind,
        index: usize,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditProfile(id))?;
        let old = self.load_member(id).await?;
        let mut new = old.record.clone();
        let member = new.member_mut(id).expect("geladen");
        let removed = match kind {
            LicenseKind::Sailing if index < member.sailing_licenses.len() => {
                member.sailing_licenses.remove(index);
                true
            }
            LicenseKind::Radio if index < member.radio_licenses.len() => {
                member.radio_licenses.remove(index);
                true
            }
            _ => false,
        };
        if !removed {
            return Err(AppError::NotFound("Eintrag".into()));
        }
        let entry = audit(
            &principal,
            "member.license.remove",
            "member",
            id,
            Some(id),
            match kind {
                LicenseKind::Sailing => "Segelschein entfernt",
                LicenseKind::Radio => "Funklizenz entfernt",
            },
        );
        self.save(&old, &new, vec![entry]).await
    }
}

fn check_names(input: &MemberInput, errors: &mut Vec<FieldError>) {
    if input.first_name.trim().is_empty() {
        errors.push(rejected("first_name", "Bitte den Vornamen angeben."));
    }
    if input.last_name.trim().is_empty() {
        errors.push(rejected("last_name", "Bitte den Nachnamen angeben."));
    }
}

/// Ein doppeltes Anmeldekonto, das erst beim Speichern auffällt (Wettlauf), als Feldfehler melden.
fn login_conflict(error: AppError) -> AppError {
    match error {
        AppError::Invalid(issues)
            if issues.iter().any(|i| {
                matches!(
                    i,
                    vereinsakte_core::issues::RecordIssue::DuplicateLoginEmail(_)
                )
            }) =>
        {
            AppError::rejected(
                "login_email",
                "Dieses Anmeldekonto gehört bereits einem anderen Mitglied.",
            )
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{ANNA, BEN, CLARA, EVA, FRANK, GRETA, INES};
    use vereinsakte_core::member::SailingLicenseType;
    use vereinsakte_core::time::club_time;

    use super::*;
    use crate::service::tests::{service, service_at};

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn contact() -> ContactInput {
        ContactInput {
            street: "Hafenstraße 1".into(),
            postal_code: "24103".into(),
            city: "Kiel".into(),
            ..Default::default()
        }
    }

    fn input(number: Option<u32>, login: &str) -> MemberInput {
        MemberInput {
            number,
            first_name: "Nora".into(),
            last_name: "Neu".into(),
            contact: contact(),
            login_email: login.into(),
            membership_type: MembershipType::Regular,
            board_consent: false,
            address_valid: true,
            child_of: None,
            joined: None,
        }
    }

    #[tokio::test]
    async fn member_page_visibility_follows_rights() {
        let (service, _) = service();
        let seen_by_greta = service.member_page(&user(GRETA), CLARA).await.unwrap();
        assert_eq!(seen_by_greta.name, "Clara Jensen");
        assert!(seen_by_greta.details.is_none());
        assert!(!seen_by_greta.can_edit_profile && !seen_by_greta.can_manage);
        let seen_by_eva = service.member_page(&user(EVA), CLARA).await.unwrap();
        assert!(seen_by_eva.details.is_some());
        assert!(seen_by_eva.can_manage);
        let own = service.member_page(&user(GRETA), GRETA).await.unwrap();
        assert!(own.details.is_some() && own.is_self && own.can_edit_profile);
    }

    #[tokio::test]
    async fn member_page_shows_contact_to_everyone_unless_blocked() {
        use vereinsakte_core::example::LUKAS;
        let (service, _) = service();
        let page = service.member_page(&user(GRETA), CLARA).await.unwrap();
        assert!(matches!(page.contact, ContactInfo::Visible { .. }));
        assert_eq!(page.member_number, 103);
        assert!(
            page.details.is_none(),
            "Anschrift und Konto bleiben verborgen"
        );

        service
            .set_contact_blocked(&user(CLARA), CLARA, true)
            .await
            .unwrap();
        let by_greta = service.member_page(&user(GRETA), CLARA).await.unwrap();
        assert_eq!(by_greta.contact, ContactInfo::Blocked);
        // Revisor liest die Details, aber nicht die gesperrten Kontaktdaten.
        let by_auditor = service.member_page(&user(LUKAS), CLARA).await.unwrap();
        let details = by_auditor.details.expect("Revisor liest Details");
        assert!(details.email.is_none() && details.phones.is_empty());
        assert_eq!(by_auditor.contact, ContactInfo::Blocked);
        // Vorstand und das Mitglied selbst sehen sie.
        for viewer in [EVA, CLARA] {
            let page = service.member_page(&user(viewer), CLARA).await.unwrap();
            assert!(matches!(page.contact, ContactInfo::Visible { .. }));
            assert!(page.details.unwrap().email.is_some());
        }
    }

    #[tokio::test]
    async fn former_member_page_is_only_for_the_authorised() {
        let (service, _) = service_at(club_time(2026, 12, 1, 12, 0));
        assert!(service.member_page(&user(EVA), INES).await.is_ok());
        let result = service.member_page(&user(GRETA), INES).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn own_contact_data_can_be_changed_and_is_logged_without_values() {
        let (service, store) = service();
        let form = service.member_form(&user(ANNA), ANNA).await.unwrap();
        let mut c = form.contact.clone();
        c.street = "Neue Straße 5".into();
        c.mobile = "0170 1234".into();
        service
            .update_contact(&user(ANNA), ANNA, form.version, &c)
            .await
            .unwrap();
        let anna = store.snapshot().member(ANNA).unwrap().clone();
        assert_eq!(anna.address.street, "Neue Straße 5");
        let log = store.audit_entries();
        assert_eq!(log.len(), 1);
        assert!(log[0].summary.contains("Anschrift") && log[0].summary.contains("Telefon"));
        assert!(!log[0].summary.contains("Neue Straße") && !log[0].summary.contains("0170"));
        // Mit dem alten Stand erneut speichern → Konflikt, nichts ändert sich.
        c.city = "Anderswo".into();
        let stale = service
            .update_contact(&user(ANNA), ANNA, form.version, &c)
            .await;
        assert_eq!(stale, Err(AppError::Conflict));
        assert_eq!(store.snapshot().member(ANNA).unwrap().address.city, "Kiel");
    }

    #[tokio::test]
    async fn contact_data_of_others_is_protected_and_validated() {
        let (service, _) = service();
        let result = service
            .update_contact(&user(GRETA), CLARA, 1, &contact())
            .await;
        assert_eq!(result, Err(AppError::Forbidden));
        let mut bad = contact();
        bad.street.clear();
        bad.email = "keine-adresse".into();
        let Err(AppError::Rejected(errors)) =
            service.update_contact(&user(ANNA), ANNA, 1, &bad).await
        else {
            panic!("Eingabe muss abgelehnt werden");
        };
        let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"street") && fields.contains(&"email"));
    }

    #[tokio::test]
    async fn board_creates_member_with_next_number_and_rejects_duplicates() {
        let (service, store) = service();
        let id = service
            .create_member(&user(EVA), &input(None, "nora.neu@sgfh.example"))
            .await
            .unwrap();
        assert_eq!(id.get(), 114);
        assert_eq!(
            store.snapshot().member(id).unwrap().login_email.as_deref(),
            Some("nora.neu@sgfh.example")
        );
        let duplicate_number = service
            .create_member(&user(EVA), &input(Some(101), ""))
            .await;
        let Err(AppError::Rejected(e)) = duplicate_number else {
            panic!()
        };
        assert_eq!(e[0].field, "number");
        let duplicate_login = service
            .create_member(&user(EVA), &input(None, "NORA.NEU@SGFH.EXAMPLE"))
            .await;
        let Err(AppError::Rejected(e)) = duplicate_login else {
            panic!()
        };
        assert_eq!(e[0].field, "login_email");
        let forbidden = service.create_member(&user(GRETA), &input(None, "")).await;
        assert_eq!(forbidden, Err(AppError::Forbidden));
        assert_eq!(store.audit_entries().len(), 1);
    }

    #[tokio::test]
    async fn board_updates_stammdaten_and_conflicts_are_detected() {
        let (service, store) = service();
        let form = service.member_form(&user(EVA), BEN).await.unwrap();
        let mut change = input(None, "");
        change.first_name = "Benjamin".into();
        change.last_name = form.last_name.clone();
        change.membership_type = MembershipType::Student;
        service
            .update_member(&user(EVA), BEN, form.version, &change)
            .await
            .unwrap();
        let ben = store.snapshot().member(BEN).unwrap().clone();
        assert_eq!(
            (ben.first_name.as_str(), ben.membership_type),
            ("Benjamin", MembershipType::Student)
        );
        let stale = service
            .update_member(&user(EVA), BEN, form.version, &change)
            .await;
        assert_eq!(stale, Err(AppError::Conflict));
        let forbidden = service
            .update_member(&user(GRETA), BEN, form.version, &change)
            .await;
        assert_eq!(forbidden, Err(AppError::Forbidden));
    }

    #[tokio::test]
    async fn resigned_member_disappears_from_directory_but_data_stays() {
        let (service, store) = service();
        let form = service.member_form(&user(EVA), GRETA).await.unwrap();
        let on = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        service
            .end_membership(&user(EVA), GRETA, form.version, on, false)
            .await
            .unwrap();
        assert!(
            store
                .snapshot()
                .member(GRETA)
                .unwrap()
                .membership
                .left_at
                .is_some()
        );
        let greta = store.snapshot().member(GRETA).unwrap().clone();
        assert!(
            !greta.is_active_at(club_time(2026, 11, 1, 12, 0)),
            "im November ausgeschieden"
        );
        assert!(
            greta.is_active_at(club_time(2026, 10, 9, 12, 0)),
            "vorher noch aktiv"
        );
        let before_join = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
        let early = service
            .end_membership(&user(EVA), ANNA, 1, before_join, false)
            .await;
        assert!(matches!(early, Err(AppError::Rejected(_))));
    }

    #[tokio::test]
    async fn members_maintain_their_own_licences_without_proof() {
        let (service, store) = service();
        service
            .add_license(&user(GRETA), GRETA, LicenseKind::Sailing, "SKS", None)
            .await
            .unwrap();
        let greta = store.snapshot().member(GRETA).unwrap().clone();
        assert!(greta.has_sailing_license(&SailingLicenseType::Sks));
        let duplicate = service
            .add_license(&user(GRETA), GRETA, LicenseKind::Sailing, "SKS", None)
            .await;
        assert!(matches!(duplicate, Err(AppError::Rejected(_))));
        // Fremde Scheine nur für den Vorstand.
        let foreign = service
            .add_license(&user(GRETA), CLARA, LicenseKind::Radio, "SRC", None)
            .await;
        assert_eq!(foreign, Err(AppError::Forbidden));
        service
            .add_license(&user(EVA), CLARA, LicenseKind::Radio, "SRC", None)
            .await
            .unwrap();
        service
            .remove_license(&user(FRANK), GRETA, LicenseKind::Sailing, 0)
            .await
            .unwrap();
        assert!(
            !store
                .snapshot()
                .member(GRETA)
                .unwrap()
                .has_sailing_license(&SailingLicenseType::Sks)
        );
        let summaries: Vec<_> = store
            .audit_entries()
            .into_iter()
            .map(|e| e.summary)
            .collect();
        assert!(summaries.contains(&"Segelschein entfernt".to_string()));
        assert!(summaries.contains(&"Segelschein hinzugefügt".to_string()));
    }
}

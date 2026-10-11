//! Sichtmodell „Meine Daten“: alles, was der Verein über das angemeldete
//! Mitglied führt – ausschließlich dessen eigene Daten.

use chrono::Datelike;
use vereinsakte_core::account::ClaimStatus;
use vereinsakte_core::money::Money;
use vereinsakte_core::work_hours::WorkSubject;

use crate::error::AppError;
use crate::format;
use crate::service::{ClubService, CurrentUser};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MyDataView {
    pub name: String,
    pub member_number: u32,
    pub membership_type: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub address_invalid: bool,
    /// (Bezeichnung, Nummer), z. B. („Mobil“, „0151 …“)
    pub phones: Vec<(&'static str, String)>,
    pub email: Option<String>,
    pub login_email: Option<String>,
    pub joined: String,
    pub sailing_licenses: Vec<String>,
    pub radio_licenses: Vec<String>,
    pub year: i32,
    pub sailing_permit: PermitStatus,
    /// Boote, für die zum aktuellen Zeitpunkt eine Bootserlaubnis besteht.
    pub boat_permits: Vec<String>,
    pub work: Vec<WorkLine>,
    pub required_hours: String,
    pub credited_hours: String,
    pub claims: Vec<ClaimLine>,
    pub payments: Vec<PaymentLine>,
    pub balance: Balance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermitStatus {
    pub granted: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkLine {
    pub date: String,
    pub hours: String,
    pub subject: String,
    pub description: String,
    pub confirmed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimState {
    Paid,
    Partial { open: String },
    Open,
    Cancelled { reason: String },
}

impl ClaimState {
    pub fn label(&self) -> String {
        match self {
            Self::Paid => "bezahlt".into(),
            Self::Partial { open } => format!("teilweise bezahlt, offen {open}"),
            Self::Open => "offen".into(),
            Self::Cancelled { reason } => format!("storniert ({reason})"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimLine {
    pub text: String,
    pub amount: String,
    pub due: Option<String>,
    pub state: ClaimState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentLine {
    pub date: String,
    pub amount: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Balance {
    Settled,
    /// Offener Betrag
    Due(String),
    Credit(String),
}

impl Balance {
    pub fn label(&self) -> String {
        match self {
            Self::Settled => "ausgeglichen".into(),
            Self::Due(amount) => format!("offen: {amount}"),
            Self::Credit(amount) => format!("Guthaben: {amount}"),
        }
    }
}

impl ClubService {
    /// Daten des angemeldeten Mitglieds für das laufende Jahr – geladen werden
    /// nur seine eigenen Daten, die Saison und die Boote.
    pub async fn my_data(&self, user: &CurrentUser) -> Result<MyDataView, AppError> {
        let now = self.now();
        let year = now.year();
        let member = self
            .store
            .member(user.member)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {}", user.member)))?
            .value;
        let year_data = self.store.member_year(member.id, year).await?;
        let permit_data = self.store.permit_history(member.id).await?;
        let boats = self.store.boats().await?;
        let member_year = year_data.view(&member, year);
        let permits = permit_data.view(&member);

        let phones = [
            ("Festnetz", &member.phone.landline),
            ("Geschäftlich", &member.phone.business),
            ("Mobil", &member.phone.mobile),
        ]
        .into_iter()
        .filter_map(|(label, number)| number.clone().map(|n| (label, n)))
        .collect();

        let sailing_permit = match member_year.sailing_permit_at(now) {
            Ok(()) => PermitStatus {
                granted: true,
                reasons: Vec::new(),
            },
            Err(reasons) => PermitStatus {
                granted: false,
                reasons: reasons.iter().map(ToString::to_string).collect(),
            },
        };
        let boat_permits = boats
            .iter()
            .filter(|b| permits.boat_permit(b.id, now).is_ok())
            .map(|b| b.name.clone())
            .collect();

        let mut entries: Vec<_> = member_year.entries().collect();
        entries.sort_by_key(|e| e.date());
        let work = entries
            .into_iter()
            .map(|e| WorkLine {
                date: format::date(e.date()),
                hours: format::hours(e.hours()),
                subject: match e.subject {
                    WorkSubject::Club => "Verein".into(),
                    WorkSubject::Boat(boat) => boats
                        .iter()
                        .find(|b| b.id == boat)
                        .map_or_else(|| boat.to_string(), |b| b.name.clone()),
                },
                description: e.description.clone().unwrap_or_default(),
                confirmed: e.is_confirmed(),
            })
            .collect();

        let account = &member.account;
        let statuses = account.claim_status();
        let claims = account
            .claims
            .iter()
            .map(|c| {
                let state = match (&c.cancelled, statuses.iter().find(|s| s.claim_id == c.id)) {
                    (Some(cancel), _) => ClaimState::Cancelled {
                        reason: cancel.reason.clone(),
                    },
                    (None, Some(status)) => claim_state(status),
                    (None, None) => ClaimState::Open,
                };
                ClaimLine {
                    text: c.text.clone(),
                    amount: c.amount.to_string(),
                    due: c.due_at.map(format::date),
                    state,
                }
            })
            .collect();
        let payments = account
            .payments
            .iter()
            .map(|p| PaymentLine {
                date: format::date(p.at),
                amount: p.amount.to_string(),
                text: p.text.clone().unwrap_or_default(),
            })
            .collect();
        let balance = account.balance();
        let balance = if balance == Money::ZERO {
            Balance::Settled
        } else if balance.is_positive() {
            Balance::Due(balance.to_string())
        } else {
            Balance::Credit((-balance).to_string())
        };

        Ok(MyDataView {
            name: member.full_name(),
            member_number: member.id.get(),
            membership_type: member.membership_type.to_string(),
            street: member.address.street.clone(),
            postal_code: member.address.postal_code.clone(),
            city: member.address.city.clone(),
            address_invalid: !member.address_valid,
            phones,
            email: member.email.clone(),
            login_email: member.login_email.clone(),
            joined: format::date(member.membership.joined_at),
            sailing_licenses: member
                .sailing_licenses
                .iter()
                .map(|l| l.kind.to_string())
                .collect(),
            radio_licenses: member
                .radio_licenses
                .iter()
                .map(|l| l.kind.to_string())
                .collect(),
            year,
            sailing_permit,
            boat_permits,
            work,
            required_hours: format::hours(member_year.required_hours()),
            credited_hours: format::hours(member_year.credited_hours()),
            claims,
            payments,
            balance,
        })
    }
}

fn claim_state(status: &ClaimStatus) -> ClaimState {
    if status.is_paid() {
        ClaimState::Paid
    } else if status.is_partially_paid() {
        ClaimState::Partial {
            open: status.open.to_string(),
        }
    } else {
        ClaimState::Open
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{ANNA, BEN, CLARA, LUKAS};

    use super::*;
    use crate::service::tests::service;

    #[tokio::test]
    async fn anna_sees_her_data() {
        let (service, _) = service();
        let view = service
            .my_data(&CurrentUser { member: ANNA })
            .await
            .unwrap();
        assert_eq!(view.member_number, 101);
        assert_eq!(view.membership_type, "ordentliches Mitglied");
        assert_eq!(view.city, "Kiel");
        assert_eq!(view.sailing_licenses, vec!["SBF See", "SKS"]);
        assert_eq!(view.radio_licenses, vec!["SRC"]);
        assert_eq!(view.balance, Balance::Credit("10,00 €".into()));
        assert_eq!(view.balance.label(), "Guthaben: 10,00 €");
        assert!(view.sailing_permit.granted);
        assert_eq!(
            view.login_email.as_deref(),
            Some("anna.petersen@sgfh.example")
        );
    }

    #[tokio::test]
    async fn clara_is_missing_hours() {
        let (service, _) = service();
        let view = service
            .my_data(&CurrentUser { member: CLARA })
            .await
            .unwrap();
        assert!(!view.sailing_permit.granted);
        assert_eq!(
            view.sailing_permit.reasons,
            vec!["es fehlen 4,5 Arbeitsstunden"]
        );
        assert_eq!(
            view.boat_permits,
            vec!["Filou", "Dipl.Ing.ling", "Harald Blauzahn"]
        );
        let flicken = view
            .work
            .iter()
            .find(|w| w.description == "Segel der Filou geflickt")
            .unwrap();
        assert!(!flicken.confirmed);
        assert_eq!(flicken.subject, "Filou");
        assert_eq!(
            (view.credited_hours.as_str(), view.required_hours.as_str()),
            ("20,5", "25")
        );
    }

    #[tokio::test]
    async fn claim_states() {
        let (service, _) = service();
        let ben = service.my_data(&CurrentUser { member: BEN }).await.unwrap();
        assert!(
            matches!(&ben.claims[0].state, ClaimState::Cancelled { reason } if reason == "Schlüssel zurückgegeben")
        );
        assert_eq!(ben.balance, Balance::Settled);
        let lukas = service
            .my_data(&CurrentUser { member: LUKAS })
            .await
            .unwrap();
        assert!(lukas.claims.iter().all(|c| c.state == ClaimState::Open));
        assert_eq!(lukas.balance.label(), "offen: 276,00 €");
    }
}

//! Finanzen: Konten, Buchungen, Storno, Außenstände, Saisons und Gebühren,
//! Jahresbeiträge, Tourabrechnung und Jahresnachweis.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use vereinsakte_core::account::{Claim, ClaimKind, Payment};
use vereinsakte_core::fees::{FeeSchedule, Season, TourRate, TypeRate};
use vereinsakte_core::ids::{ClaimId, MemberId, PaymentId, ReservationId};
use vereinsakte_core::member::TariffGroup;
use vereinsakte_core::money::Money;
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::{Timestamp, club_time};

use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::members::{membership_types, parse_membership_type};
use crate::my_data::{Balance, ClaimLine, MyDataView, PaymentLine};
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser};

fn midnight(date: NaiveDate) -> Timestamp {
    club_time(date.year(), date.month(), date.day(), 0, 0)
}

fn trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

// ---- Konten ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountView {
    pub member: u32,
    pub name: String,
    pub is_self: bool,
    pub balance: Balance,
    pub claims: Vec<ClaimLine>,
    pub payments: Vec<PaymentLine>,
    pub can_post: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutstandingRow {
    pub member: u32,
    pub name: String,
    pub open: String,
    pub overdue: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutstandingView {
    pub at: NaiveDate,
    pub only_overdue: bool,
    pub rows: Vec<OutstandingRow>,
    pub total_open: String,
    pub total_overdue: String,
    pub can_post: bool,
}

impl ClubService {
    async fn account_data(&self, member: MemberId) -> Result<MyDataView, AppError> {
        self.member_data(member).await
    }

    /// Konto eines Mitglieds: das eigene oder – für Vorstand und Revisoren – ein fremdes.
    pub async fn account_view(
        &self,
        user: &CurrentUser,
        member: MemberId,
    ) -> Result<AccountView, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAccount(member))?;
        let data = self.account_data(member).await?;
        Ok(AccountView {
            member: member.get(),
            name: data.name,
            is_self: member == user.member,
            balance: data.balance,
            claims: data.claims,
            payments: data.payments,
            can_post: principal.can(&Capability::PostFinance),
        })
    }

    /// Außenstände zum Stichtag (Standard: heute), optional nur Überfällige.
    pub async fn outstanding_view(
        &self,
        user: &CurrentUser,
        at: Option<NaiveDate>,
        only_overdue: bool,
    ) -> Result<OutstandingView, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAllAccounts)?;
        let date = at.unwrap_or_else(|| principal.now.date_naive());
        let scoped = self
            .load(&Scope {
                members: MemberScope::All,
                ..Scope::default()
            })
            .await?;
        let record = &scoped.record;
        let t = club_time(date.year(), date.month(), date.day(), 23, 59);
        let mut rows: Vec<_> = record
            .outstanding(t)
            .into_iter()
            .filter(|o| !only_overdue || o.overdue.is_positive())
            .collect();
        rows.sort_by_key(|o| std::cmp::Reverse(o.open));
        let total_open: Money = rows.iter().map(|o| o.open).sum();
        let total_overdue: Money = rows.iter().map(|o| o.overdue).sum();
        Ok(OutstandingView {
            at: date,
            only_overdue,
            rows: rows
                .into_iter()
                .map(|o| OutstandingRow {
                    member: o.member.get(),
                    name: record
                        .member(o.member)
                        .map_or_else(|| o.member.to_string(), |m| m.full_name()),
                    open: o.open.to_string(),
                    overdue: o.overdue.to_string(),
                })
                .collect(),
            total_open: total_open.to_string(),
            total_overdue: total_overdue.to_string(),
            can_post: principal.can(&Capability::PostFinance),
        })
    }

    async fn load_account_member(
        &self,
        member: MemberId,
    ) -> Result<crate::changes::Scoped, AppError> {
        let scoped = self
            .load(&Scope {
                members: MemberScope::Some(vec![member]),
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(member).is_none() {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied gibt es nicht.",
            ));
        }
        Ok(scoped)
    }

    /// Bucht eine Einzahlung.
    pub async fn post_payment(
        &self,
        user: &CurrentUser,
        member: MemberId,
        amount: Money,
        on: Option<NaiveDate>,
        text: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        if !amount.is_positive() {
            return Err(AppError::rejected(
                "amount",
                "Der Betrag muss größer als 0 sein.",
            ));
        }
        let scoped = self.load_account_member(member).await?;
        let mut new = scoped.record.clone();
        let payment = Payment {
            id: PaymentId::new(),
            amount,
            at: on.map_or(principal.now, midnight),
            text: trimmed(text),
        };
        let id = payment.id;
        new.member_mut(member)
            .expect("geladen")
            .account
            .payments
            .push(payment);
        let entry = audit(
            &principal,
            "payment.post",
            "payment",
            id,
            Some(member),
            "Einzahlung gebucht",
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Legt eine sonstige Forderung an.
    pub async fn post_claim(
        &self,
        user: &CurrentUser,
        member: MemberId,
        amount: Money,
        text: &str,
        due: Option<NaiveDate>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        let mut errors = vec![];
        if !amount.is_positive() {
            errors.push(FieldError::new(
                "amount",
                "Der Betrag muss größer als 0 sein.",
            ));
        }
        let Some(text) = trimmed(text) else {
            errors.push(FieldError::new("text", "Bitte einen Text angeben."));
            return Err(AppError::Rejected(errors));
        };
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let scoped = self.load_account_member(member).await?;
        let mut new = scoped.record.clone();
        let claim = Claim::new(
            ClaimKind::Other,
            amount,
            text,
            principal.now,
            due.map(midnight),
        );
        let id = claim.id;
        new.member_mut(member)
            .expect("geladen")
            .account
            .claims
            .push(claim);
        let entry = audit(
            &principal,
            "claim.post",
            "claim",
            id,
            Some(member),
            "Forderung angelegt",
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Storniert eine Forderung mit Begründung; sie bleibt sichtbar.
    pub async fn cancel_claim(
        &self,
        user: &CurrentUser,
        member: MemberId,
        claim: ClaimId,
        reason: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        let Some(reason) = trimmed(reason) else {
            return Err(AppError::rejected(
                "reason",
                "Bitte eine Begründung angeben.",
            ));
        };
        let scoped = self.load_account_member(member).await?;
        let mut new = scoped.record.clone();
        let found = new
            .member_mut(member)
            .expect("geladen")
            .account
            .claims
            .iter_mut()
            .find(|c| c.id == claim)
            .ok_or_else(|| AppError::NotFound("Forderung".into()))?;
        if found.is_cancelled() {
            return Err(AppError::rejected(
                "",
                "Die Forderung ist bereits storniert.",
            ));
        }
        found.cancel(principal.now, reason);
        let entry = audit(
            &principal,
            "claim.cancel",
            "claim",
            claim,
            Some(member),
            "Forderung storniert",
        );
        self.save(&scoped, &new, vec![entry]).await
    }
}

// ---- Saisons und Gebühren --------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonRow {
    pub year: i32,
    pub hours_quota: String,
    pub rate: String,
    pub custom_fees: bool,
}

/// Ein Eingabefeld der Gebührentabelle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeField {
    pub key: String,
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonForm {
    pub year: i32,
    pub hours_quota: String,
    pub compensation_rate: String,
    pub annual: Vec<FeeField>,
    pub admission: Vec<FeeField>,
    pub tour: Vec<FeeField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonsView {
    pub seasons: Vec<SeasonRow>,
    pub form: SeasonForm,
    pub can_edit: bool,
}

fn group_label(group: TariffGroup) -> &'static str {
    match group {
        TariffGroup::Regular => "ordentliche Mitglieder",
        TariffGroup::Student => "studentische Mitglieder",
        TariffGroup::Extraordinary => "außerordentliche Mitglieder",
        TariffGroup::Guest => "Gäste",
    }
}

const GROUPS: [(TariffGroup, &str); 4] = [
    (TariffGroup::Regular, "regular"),
    (TariffGroup::Student, "student"),
    (TariffGroup::Extraordinary, "extraordinary"),
    (TariffGroup::Guest, "guest"),
];

fn money_text(m: Money) -> String {
    m.to_string().replace(" €", "")
}

fn season_form(year: i32, season: Option<&Season>) -> SeasonForm {
    let fees = vereinsakte_core::fees::fees(season);
    let quota = vereinsakte_core::fees::hours_quota(season);
    let rate = vereinsakte_core::fees::compensation_rate(season);
    let types = membership_types();
    let field = |prefix: &str, key: &str, label: &str, value: String| FeeField {
        key: format!("{prefix}_{key}"),
        label: label.to_string(),
        value,
    };
    let annual = types
        .iter()
        .map(|(k, label)| {
            let t = parse_membership_type(k).expect("bekannt");
            field("annual", k, label, money_text(fees.annual_fee(t)))
        })
        .collect();
    let admission = types
        .iter()
        .map(|(k, label)| {
            let t = parse_membership_type(k).expect("bekannt");
            field("admission", k, label, money_text(fees.admission_fee(t)))
        })
        .collect();
    let mut tour = vec![];
    for (group, key) in GROUPS {
        let rate = fees.tour_rate(group);
        let label = group_label(group);
        tour.push(field(
            "tour",
            &format!("{key}_day"),
            &format!("{label}: je Tag"),
            rate.map(|r| money_text(r.day)).unwrap_or_default(),
        ));
        tour.push(field(
            "tour",
            &format!("{key}_week"),
            &format!("{label}: je Woche"),
            rate.map(|r| money_text(r.week)).unwrap_or_default(),
        ));
        tour.push(field(
            "tour",
            &format!("{key}_first"),
            &format!("{label}: erste Woche (optional)"),
            rate.and_then(|r| r.first_week)
                .map(money_text)
                .unwrap_or_default(),
        ));
    }
    SeasonForm {
        year,
        hours_quota: format::hours(quota),
        compensation_rate: money_text(rate),
        annual,
        admission,
        tour,
    }
}

impl ClubService {
    /// Saisons mit Gebührensätzen; `year` wählt die im Formular gezeigte Saison.
    pub async fn seasons_view(
        &self,
        user: &CurrentUser,
        year: Option<i32>,
    ) -> Result<SeasonsView, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAllAccounts)?;
        let scoped = self
            .load(&Scope {
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let mut seasons: Vec<&Season> = scoped.record.seasons.iter().collect();
        seasons.sort_by_key(|s| s.year);
        let year = year.unwrap_or_else(|| principal.now.year() + 1);
        let form = season_form(year, scoped.record.season(year));
        Ok(SeasonsView {
            seasons: seasons
                .into_iter()
                .map(|s| SeasonRow {
                    year: s.year,
                    hours_quota: format::hours(vereinsakte_core::fees::hours_quota(Some(s))),
                    rate: vereinsakte_core::fees::compensation_rate(Some(s)).to_string(),
                    custom_fees: s.fees.is_some(),
                })
                .collect(),
            form,
            can_edit: principal.can(&Capability::PostFinance),
        })
    }

    /// Speichert Soll-Stunden, Stundensatz und Gebührentabelle einer Saison.
    /// Bereits angelegte Forderungen ändern sich dadurch nicht.
    pub async fn save_season(
        &self,
        user: &CurrentUser,
        year: i32,
        fields: &HashMap<String, String>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        let mut errors = vec![];
        let money = |key: &str, required: bool, errors: &mut Vec<FieldError>| -> Option<Money> {
            let text = fields
                .get(key)
                .map_or("", String::as_str)
                .trim()
                .replace(',', ".");
            if text.is_empty() {
                if required {
                    errors.push(FieldError::new(key, "Bitte einen Betrag angeben."));
                }
                return None;
            }
            match text.parse::<Money>() {
                Ok(m) if m >= Money::ZERO => Some(m),
                _ => {
                    errors.push(FieldError::new(
                        key,
                        "Bitte einen Betrag wie 12,50 angeben.",
                    ));
                    None
                }
            }
        };
        let quota = fields
            .get("hours_quota")
            .and_then(|t| format::parse_decimal(t))
            .filter(|q| *q >= 0.0);
        if quota.is_none() {
            errors.push(FieldError::new(
                "hours_quota",
                "Bitte die Soll-Stunden als Zahl angeben.",
            ));
        }
        let rate = money("compensation_rate", true, &mut errors);
        let mut annual = vec![];
        let mut admission = vec![];
        for (key, _) in membership_types() {
            let t = parse_membership_type(key).expect("bekannt");
            if let Some(amount) = money(&format!("annual_{key}"), true, &mut errors) {
                annual.push(TypeRate {
                    membership_type: t,
                    amount,
                });
            }
            if let Some(amount) = money(&format!("admission_{key}"), true, &mut errors) {
                admission.push(TypeRate {
                    membership_type: t,
                    amount,
                });
            }
        }
        let mut tour = vec![];
        for (group, key) in GROUPS {
            let day = money(&format!("tour_{key}_day"), true, &mut errors);
            let week = money(&format!("tour_{key}_week"), true, &mut errors);
            let first = money(&format!("tour_{key}_first"), false, &mut errors);
            if let (Some(day), Some(week)) = (day, week) {
                tour.push(TourRate {
                    tariff_group: group,
                    day,
                    week,
                    first_week: first,
                });
            }
        }
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let scoped = self
            .load(&Scope {
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let mut new = scoped.record.clone();
        let season = Season {
            year,
            hours_quota: quota,
            compensation_rate: rate,
            fees: Some(FeeSchedule {
                annual_fee: annual,
                admission_fee: admission,
                tour,
            }),
        };
        match new.seasons.iter_mut().find(|s| s.year == year) {
            Some(slot) => *slot = season,
            None => new.seasons.push(season),
        }
        let entry = audit(
            &principal,
            "season.save",
            "season",
            year,
            None,
            format!("Saison {year} gespeichert"),
        );
        self.save(&scoped, &new, vec![entry]).await
    }
}

// ---- Jahresbeiträge ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeePreviewLine {
    pub member: u32,
    pub name: String,
    pub text: String,
    pub amount: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnualFeePreview {
    pub year: i32,
    pub lines: Vec<FeePreviewLine>,
    /// Mitglieder, die den Beitrag der Saison schon haben.
    pub existing: usize,
    pub total: String,
    pub can_create: bool,
}

impl ClubService {
    async fn annual_scope(&self) -> Scope {
        Scope::all_members()
    }

    /// Vorschau der noch fehlenden Jahresbeiträge einer Saison.
    pub async fn annual_fee_preview(
        &self,
        user: &CurrentUser,
        year: Option<i32>,
    ) -> Result<AnnualFeePreview, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAllAccounts)?;
        let year = year.unwrap_or_else(|| principal.now.year() + 1);
        let scoped = self.load(&self.annual_scope().await).await?;
        let record = &scoped.record;
        let claims = record.annual_fee_claims(year, principal.now);
        let kind = ClaimKind::AnnualFee { year };
        let existing = record
            .members
            .iter()
            .filter(|m| m.account.has_claim(&kind))
            .count();
        let total: Money = claims.iter().map(|(_, c)| c.amount).sum();
        Ok(AnnualFeePreview {
            year,
            lines: claims
                .iter()
                .map(|(m, c)| FeePreviewLine {
                    member: m.get(),
                    name: record
                        .member(*m)
                        .map_or_else(|| m.to_string(), |x| x.full_name()),
                    text: c.text.clone(),
                    amount: c.amount.to_string(),
                })
                .collect(),
            existing,
            total: total.to_string(),
            can_create: principal.can(&Capability::PostFinance),
        })
    }

    /// Legt die fehlenden Jahresbeiträge an (ohne Doppelungen); liefert die Anzahl.
    pub async fn create_annual_fees(
        &self,
        user: &CurrentUser,
        year: i32,
    ) -> Result<usize, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        let scoped = self.load(&self.annual_scope().await).await?;
        let mut new = scoped.record.clone();
        let claims = new.annual_fee_claims(year, principal.now);
        let count = new.add_claims(claims);
        if count == 0 {
            return Ok(0);
        }
        let entry = audit(
            &principal,
            "fees.annual",
            "season",
            year,
            None,
            format!("{count} Jahresbeiträge {year} angelegt"),
        );
        self.save(&scoped, &new, vec![entry]).await?;
        Ok(count)
    }
}

// ---- Tourabrechnung ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TourRow {
    pub id: String,
    pub boat: String,
    pub when: String,
    pub skipper: String,
    pub skipper_id: u32,
    pub billed: usize,
    pub missing: usize,
    pub total: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TourBilling {
    pub rows: Vec<TourRow>,
    pub can_bill: bool,
}

impl ClubService {
    /// Alle Touren (vergangene zuerst) mit Stand der Abrechnung.
    pub async fn tour_billing(&self, user: &CurrentUser) -> Result<TourBilling, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAllAccounts)?;
        let scoped = self
            .load(&Scope {
                members: MemberScope::All,
                boats: BoatScope::All,
                reservations_between: Some((
                    club_time(2000, 1, 1, 0, 0),
                    club_time(2100, 1, 1, 0, 0),
                )),
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let record = &scoped.record;
        let mut tours: Vec<_> = record.reservations.iter().filter(|r| r.is_tour()).collect();
        tours.sort_by_key(|r| r.start);
        let name = |m: MemberId| {
            record
                .member(m)
                .map_or_else(|| m.to_string(), |x| x.full_name())
        };
        let rows = tours
            .into_iter()
            .map(|r| {
                let billed = record.tour_fee_overview(r);
                let missing = record.tour_fee_claims(r, principal.now);
                let total: Money = billed.iter().map(|l| l.amount).sum::<Money>()
                    + missing.iter().map(|(_, c)| c.amount).sum::<Money>();
                TourRow {
                    id: r.id.to_string(),
                    boat: record
                        .boats
                        .iter()
                        .find(|b| b.id == r.boat)
                        .map_or_else(String::new, |b| b.name.clone()),
                    when: format!("{} – {}", format::date(r.start), format::date(r.end)),
                    skipper: name(r.skipper),
                    skipper_id: r.skipper.get(),
                    billed: billed.len(),
                    missing: missing.len(),
                    total: total.to_string(),
                }
            })
            .collect();
        Ok(TourBilling {
            rows,
            can_bill: principal.can(&Capability::PostFinance),
        })
    }

    /// Legt die fehlenden Tourgebühren-Forderungen an (ohne Doppelungen).
    pub async fn bill_tour(
        &self,
        user: &CurrentUser,
        id: ReservationId,
    ) -> Result<usize, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::PostFinance)?;
        let scoped = self
            .load(&Scope {
                members: MemberScope::All,
                boats: BoatScope::All,
                reservations_between: Some((
                    club_time(2000, 1, 1, 0, 0),
                    club_time(2100, 1, 1, 0, 0),
                )),
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let mut new = scoped.record.clone();
        let reservation = new
            .reservation(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("Reservierung".into()))?;
        if !reservation.is_tour() {
            return Err(AppError::rejected("", "Diese Reservierung ist keine Tour."));
        }
        let claims = new.tour_fee_claims(&reservation, principal.now);
        let count = new.add_claims(claims);
        if count == 0 {
            return Ok(0);
        }
        let entry = audit(
            &principal,
            "fees.tour",
            "reservation",
            id,
            Some(reservation.skipper),
            format!("{count} Tourgebühren angelegt"),
        );
        self.save(&scoped, &new, vec![entry]).await?;
        Ok(count)
    }
}

// ---- Jahresnachweis ------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnualProof {
    pub member: u32,
    pub name: String,
    pub number: u32,
    pub year: i32,
    pub membership_type: String,
    pub required: String,
    pub credited: String,
    pub fee_state: String,
    pub permit_granted: bool,
    pub permit_reasons: Vec<String>,
    pub boat_permits: Vec<String>,
}

impl ClubService {
    /// Jahresnachweis eines Mitglieds (Vorstand, Revisoren; das Mitglied selbst).
    pub async fn annual_proof(
        &self,
        user: &CurrentUser,
        member: MemberId,
        year: Option<i32>,
    ) -> Result<AnnualProof, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadMember(member))?;
        let year = year.unwrap_or_else(|| principal.now.year());
        let scoped = self
            .load(&Scope {
                members: MemberScope::Some(vec![member]),
                boats: BoatScope::All,
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let record = &scoped.record;
        let my = record
            .member_year(member, year)
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {member}")))?;
        let (granted, reasons) = match my.sailing_permit() {
            Ok(()) => (true, vec![]),
            Err(r) => (false, r.iter().map(ToString::to_string).collect()),
        };
        let fee_state = match my.fee_arrears() {
            None => "keine Beitragspflicht".to_string(),
            Some(a) if a.is_positive() => format!("Rückstand {a}"),
            Some(_) => "beglichen".to_string(),
        };
        let at = principal.now;
        let boat_permits = record
            .boats
            .iter()
            .filter(|b| record.boat_permit(member, b.id, at).is_ok())
            .map(|b| b.name.clone())
            .collect();
        let m = my.member;
        Ok(AnnualProof {
            member: member.get(),
            name: m.full_name(),
            number: member.get(),
            year,
            membership_type: m.membership_type.to_string(),
            required: format::hours(my.required_hours()),
            credited: format::hours(my.credited_hours()),
            fee_state,
            permit_granted: granted,
            permit_reasons: reasons,
            boat_permits,
        })
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{ANNA, BEN, CLARA, DEHLER_TOUR, EVA, GRETA, JONAS, LUKAS};

    use super::*;
    use crate::service::tests::service;

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn eur(text: &str) -> Money {
        text.parse().unwrap()
    }

    #[tokio::test]
    async fn accounts_are_visible_to_owner_board_and_auditors_only() {
        let (service, _) = service();
        let anna = service.account_view(&user(ANNA), ANNA).await.unwrap();
        assert_eq!(anna.balance.label(), "Guthaben: 10,00 €");
        assert!(anna.is_self && anna.can_post, "Anna gehört zum Vorstand");
        let lukas = service.account_view(&user(LUKAS), LUKAS).await.unwrap();
        assert_eq!(lukas.balance.label(), "offen: 276,00 €");
        assert_eq!(
            service.account_view(&user(GRETA), CLARA).await,
            Err(AppError::Forbidden)
        );
        let seen_by_auditor = service.account_view(&user(LUKAS), CLARA).await.unwrap();
        assert!(!seen_by_auditor.can_post, "Revisor liest nur");
        assert!(
            service
                .account_view(&user(JONAS), CLARA)
                .await
                .unwrap()
                .can_post
        );
    }

    #[tokio::test]
    async fn payments_claims_and_cancellations_follow_rights_and_validation() {
        let (service, store) = service();
        service
            .post_payment(
                &user(JONAS),
                ANNA,
                eur("240"),
                NaiveDate::from_ymd_opt(2026, 4, 3),
                "Beitrag",
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .account_view(&user(ANNA), ANNA)
                .await
                .unwrap()
                .balance
                .label(),
            "Guthaben: 250,00 €"
        );
        assert_eq!(
            service
                .post_payment(&user(LUKAS), ANNA, eur("5"), None, "")
                .await,
            Err(AppError::Forbidden)
        );
        let negative = service
            .post_payment(&user(JONAS), ANNA, eur("-5"), None, "")
            .await;
        assert!(matches!(negative, Err(AppError::Rejected(_))));
        service
            .post_claim(&user(EVA), ANNA, eur("20"), "Schlüsselpfand", None)
            .await
            .unwrap();
        let snapshot = store.snapshot();
        let claim = snapshot
            .member(ANNA)
            .unwrap()
            .account
            .claims
            .iter()
            .find(|c| c.text == "Schlüsselpfand")
            .unwrap()
            .id;
        let empty = service.cancel_claim(&user(JONAS), ANNA, claim, " ").await;
        assert!(matches!(empty, Err(AppError::Rejected(_))));
        service
            .cancel_claim(&user(JONAS), ANNA, claim, "Schlüssel zurückgegeben")
            .await
            .unwrap();
        let view = service.account_view(&user(ANNA), ANNA).await.unwrap();
        let line = view
            .claims
            .iter()
            .find(|c| c.text == "Schlüsselpfand")
            .unwrap();
        assert!(
            line.state.label().contains("storniert")
                && line.state.label().contains("Schlüssel zurückgegeben")
        );
        assert_eq!(
            view.balance.label(),
            "Guthaben: 250,00 €",
            "Storno zählt nicht"
        );
        let twice = service
            .cancel_claim(&user(JONAS), ANNA, claim, "nochmal")
            .await;
        assert!(matches!(twice, Err(AppError::Rejected(_))));
        let actions: Vec<_> = store
            .audit_entries()
            .into_iter()
            .map(|e| e.action)
            .collect();
        assert_eq!(actions, ["payment.post", "claim.post", "claim.cancel"]);
    }

    #[tokio::test]
    async fn outstanding_lists_overdue_amounts_at_the_cut_off_date() {
        let (service, _) = service();
        let all = service
            .outstanding_view(&user(JONAS), None, false)
            .await
            .unwrap();
        let names: Vec<_> = all.rows.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Lukas Meier") && names.contains(&"Clara Jensen"));
        assert_eq!(all.rows[0].name, "Lukas Meier", "größter Betrag zuerst");
        assert_eq!(all.rows[0].open, "276,00 €");
        let early = service
            .outstanding_view(&user(JONAS), NaiveDate::from_ymd_opt(2026, 1, 1), true)
            .await
            .unwrap();
        assert!(early.rows.iter().all(|r| r.overdue != "0,00 €"));
        assert_eq!(
            service.outstanding_view(&user(GRETA), None, false).await,
            Err(AppError::Forbidden)
        );
        assert!(
            !service
                .outstanding_view(&user(LUKAS), None, false)
                .await
                .unwrap()
                .can_post
        );
    }

    #[tokio::test]
    async fn season_rates_can_be_changed_without_touching_existing_claims() {
        let (service, store) = service();
        let before = store
            .snapshot()
            .member(ANNA)
            .unwrap()
            .account
            .claims
            .clone();
        let view = service
            .seasons_view(&user(JONAS), Some(2027))
            .await
            .unwrap();
        let mut fields: HashMap<String, String> = HashMap::new();
        fields.insert("hours_quota".into(), view.form.hours_quota.clone());
        fields.insert(
            "compensation_rate".into(),
            view.form.compensation_rate.clone(),
        );
        for f in view
            .form
            .annual
            .iter()
            .chain(&view.form.admission)
            .chain(&view.form.tour)
        {
            fields.insert(f.key.clone(), f.value.clone());
        }
        fields.insert("annual_regular".into(), "260,00".into());
        service
            .save_season(&user(JONAS), 2027, &fields)
            .await
            .unwrap();
        let after = service
            .seasons_view(&user(JONAS), Some(2027))
            .await
            .unwrap();
        assert_eq!(
            after
                .form
                .annual
                .iter()
                .find(|f| f.key == "annual_regular")
                .unwrap()
                .value,
            "260,00"
        );
        assert!(
            after
                .seasons
                .iter()
                .any(|s| s.year == 2027 && s.custom_fees)
        );
        assert_eq!(
            store.snapshot().member(ANNA).unwrap().account.claims,
            before
        );
        fields.insert("annual_student".into(), "abc".into());
        let Err(AppError::Rejected(e)) = service.save_season(&user(JONAS), 2027, &fields).await
        else {
            panic!()
        };
        assert_eq!(e[0].field, "annual_student");
        assert_eq!(
            service.save_season(&user(LUKAS), 2027, &fields).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn annual_fees_are_created_once_per_member() {
        let (service, store) = service();
        let preview = service
            .annual_fee_preview(&user(JONAS), Some(2027))
            .await
            .unwrap();
        assert!(!preview.lines.is_empty() && preview.lines.iter().all(|l| l.text.contains("2027")));
        let count = service
            .create_annual_fees(&user(JONAS), 2027)
            .await
            .unwrap();
        assert_eq!(count, preview.lines.len());
        let again = service
            .annual_fee_preview(&user(JONAS), Some(2027))
            .await
            .unwrap();
        assert!(again.lines.is_empty() && again.existing >= count);
        assert_eq!(
            service
                .create_annual_fees(&user(JONAS), 2027)
                .await
                .unwrap(),
            0
        );
        let fees: usize = store
            .snapshot()
            .members
            .iter()
            .flat_map(|m| &m.account.claims)
            .filter(|c| matches!(c.kind, ClaimKind::AnnualFee { year: 2027 }))
            .count();
        assert_eq!(fees, count);
        assert_eq!(
            service.create_annual_fees(&user(BEN), 2027).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn tours_are_billed_without_duplicates() {
        let (service, _) = service();
        let overview = service.tour_billing(&user(JONAS)).await.unwrap();
        assert!(
            overview
                .rows
                .iter()
                .any(|r| r.id == DEHLER_TOUR.to_string())
        );
        assert_eq!(
            service.bill_tour(&user(JONAS), DEHLER_TOUR).await.unwrap(),
            0,
            "Beispiel ist abgerechnet"
        );
        let plain = vereinsakte_core::example::JOLLE_SAMSTAG;
        assert!(matches!(
            service.bill_tour(&user(JONAS), plain).await,
            Err(AppError::Rejected(_))
        ));
        assert_eq!(
            service.bill_tour(&user(GRETA), DEHLER_TOUR).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn annual_proof_shows_hours_fees_and_permit() {
        let (service, _) = service();
        let proof = service
            .annual_proof(&user(JONAS), CLARA, None)
            .await
            .unwrap();
        assert_eq!(
            (proof.required.as_str(), proof.credited.as_str()),
            ("25", "20,5")
        );
        assert!(!proof.permit_granted && proof.permit_reasons.iter().any(|r| r.contains("4,5")));
        assert!(proof.boat_permits.contains(&"Filou".to_string()));
        assert_eq!(
            service.annual_proof(&user(GRETA), CLARA, None).await,
            Err(AppError::Forbidden)
        );
        assert!(
            service
                .annual_proof(&user(CLARA), CLARA, None)
                .await
                .is_ok()
        );
    }
}

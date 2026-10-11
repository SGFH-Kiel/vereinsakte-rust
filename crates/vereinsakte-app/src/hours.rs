//! Stundenübersicht, Überträge und finanzielle Abgeltung (Vorstand).

use chrono::Datelike;
use vereinsakte_core::ids::{HourTransferId, MemberId};
use vereinsakte_core::money::Money;
use vereinsakte_core::rights::Capability;
use vereinsakte_core::sailing_permit::{AnnualRecord, Compensation};
use vereinsakte_core::work_hours::HourTransfer;

use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser};
use crate::work::{HoursOverview, HoursRow, trimmed};

impl ClubService {
    /// Stunden aller aktiven Mitglieder im Jahr; optional nur mit Rückstand.
    pub async fn hours_overview(
        &self,
        user: &CurrentUser,
        year: Option<i32>,
        only_deficit: bool,
    ) -> Result<HoursOverview, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadAllHours)?;
        let year = year.unwrap_or_else(|| principal.now.year());
        let scoped = self
            .load(&Scope {
                members: MemberScope::All,
                seasons: true,
                ..Scope::default()
            })
            .await?;
        let record = &scoped.record;
        let mut rows: Vec<HoursRow> = record
            .members
            .iter()
            .filter(|m| m.is_active_in(year))
            .filter_map(|m| {
                let required = record.required_hours(m.id, year);
                let credited = record.credited_hours(m.id, year);
                let missing = (required - credited).max(0.0);
                if only_deficit && missing <= 0.0 {
                    return None;
                }
                Some(HoursRow {
                    id: m.id.get(),
                    name: format!("{}, {}", m.last_name, m.first_name),
                    required: format::hours(required),
                    credited: format::hours(credited),
                    missing: (missing > 0.0)
                        .then(|| format!("es fehlen {} Arbeitsstunden", format::hours(missing))),
                })
            })
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        let mut members: Vec<(u32, String)> = record
            .members
            .iter()
            .filter(|m| m.is_active_at(principal.now))
            .map(|m| (m.id.get(), m.full_name()))
            .collect();
        members.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(HoursOverview {
            year,
            rows,
            can_grant: principal.can(&Capability::GrantHours),
            members,
            rate: record.compensation_rate(year).to_string(),
        })
    }

    /// Überträgt Stunden von einem Mitglied an ein anderes (Vorstand).
    pub async fn transfer_hours(
        &self,
        user: &CurrentUser,
        from: MemberId,
        to: MemberId,
        year: i32,
        hours: &str,
        description: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::GrantHours)?;
        let Some(hours) = format::parse_decimal(hours) else {
            return Err(AppError::rejected(
                "hours",
                "Bitte eine Stundenzahl eingeben (z. B. 4,5).",
            ));
        };
        let transfer = HourTransfer {
            id: HourTransferId::new(),
            year,
            from,
            to,
            hours,
            description: trimmed(description),
        };
        if let Err(errors) = transfer.validate() {
            return Err(AppError::Rejected(
                errors
                    .into_iter()
                    .map(|e| FieldError::new("hours", e.to_string()))
                    .collect(),
            ));
        }
        let scoped = self
            .load(&Scope {
                members: MemberScope::Some(vec![from, to]),
                boats: BoatScope::None,
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(from).is_none() || scoped.record.member(to).is_none() {
            return Err(AppError::rejected("to", "Unbekanntes Mitglied."));
        }
        let id = transfer.id;
        let mut new = scoped.record.clone();
        new.hour_transfers.push(transfer);
        let log = audit(
            &principal,
            "hours.transfer",
            "hour_transfer",
            id,
            Some(to),
            "Stundenübertrag erfasst",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Genehmigt die Abgeltung von Stunden; die Forderung entsteht im Konto.
    /// Ohne Betrag gilt Stunden × Stundensatz der Saison.
    pub async fn approve_compensation(
        &self,
        user: &CurrentUser,
        member: MemberId,
        year: i32,
        hours: &str,
        amount: Option<Money>,
        reason: &str,
    ) -> Result<Money, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::GrantHours)?;
        let Some(hours) = format::parse_decimal(hours).filter(|h| *h > 0.0) else {
            return Err(AppError::rejected(
                "hours",
                "Bitte eine positive Stundenzahl eingeben.",
            ));
        };
        if amount.is_some_and(|a| !a.is_positive()) {
            return Err(AppError::rejected(
                "amount",
                "Der Betrag muss größer als 0 sein.",
            ));
        }
        let scoped = self
            .load(&Scope {
                members: MemberScope::Some(vec![member]),
                seasons: true,
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(member).is_none() {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied gibt es nicht.",
            ));
        }
        let mut new = scoped.record.clone();
        let compensation = Compensation {
            hours,
            amount,
            approved_at: principal.now,
            reason: trimmed(reason),
        };
        match new
            .annual_records
            .iter_mut()
            .find(|r| r.member == member && r.year == year)
        {
            Some(record) => {
                if record.compensation.is_some() {
                    return Err(AppError::rejected(
                        "",
                        "Für dieses Jahr ist bereits eine Abgeltung genehmigt.",
                    ));
                }
                record.compensation = Some(compensation.clone());
            }
            None => new.annual_records.push(AnnualRecord {
                member,
                year,
                reduced_quota: None,
                compensation: Some(compensation.clone()),
            }),
        }
        let total = new.compensation_amount(year, &compensation);
        let claims = new.compensation_claims(principal.now);
        new.add_claims(claims);
        let log = audit(
            &principal,
            "hours.compensation",
            "annual_record",
            format!("{member}:{year}"),
            Some(member),
            "Abgeltung genehmigt",
        );
        self.save(&scoped, &new, vec![log]).await?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{ANNA, BEN, CLARA, EVA, FRANK, GRETA, HARALD_BLAUZAHN};
    use vereinsakte_core::task::TaskStatus;

    use super::*;
    use crate::service::tests::service;
    use crate::work::{EntryInput, EventInput, TaskInput};

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn input(hours: &str) -> EntryInput {
        EntryInput {
            date: "2026-10-01".into(),
            hours: hours.into(),
            subject: HARALD_BLAUZAHN.to_string(),
            description: "Antifouling".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn member_records_work_and_sees_it_unconfirmed() {
        let (service, store) = service();
        let id = service
            .add_work_entry(&user(CLARA), &input("3"))
            .await
            .unwrap();
        let view = service.work_view(&user(CLARA), None, None).await.unwrap();
        let line = view
            .entries
            .iter()
            .find(|e| e.id == id.to_string())
            .unwrap();
        assert!(!line.confirmed && line.can_edit && line.hours == "3");
        assert_eq!(view.credited, "20,5", "unbestätigt zählt nicht");
        assert_eq!(store.audit_entries().last().unwrap().action, "work.add");
    }

    #[tokio::test]
    async fn invalid_entries_are_rejected_per_field() {
        let (service, _) = service();
        let mut both = input("3");
        both.start = "08:00".into();
        both.end = "10:00".into();
        let Err(AppError::Rejected(e)) = service.add_work_entry(&user(CLARA), &both).await else {
            panic!()
        };
        assert_eq!(e[0].field, "hours");
        let none = EntryInput {
            hours: String::new(),
            ..input("")
        };
        assert!(matches!(
            service.add_work_entry(&user(CLARA), &none).await,
            Err(AppError::Rejected(_))
        ));
        let mut bad_subject = input("2");
        bad_subject.subject = "nirgends".into();
        let Err(AppError::Rejected(e)) = service.add_work_entry(&user(CLARA), &bad_subject).await
        else {
            panic!()
        };
        assert_eq!(e[0].field, "subject");
        let mut period = input("");
        period.start = "10:00".into();
        period.end = "09:00".into();
        let Err(AppError::Rejected(e)) = service.add_work_entry(&user(CLARA), &period).await else {
            panic!()
        };
        assert_eq!(e[0].field, "end");
        let mut period = input("");
        period.start = "08:00".into();
        period.end = "10:30".into();
        service.add_work_entry(&user(CLARA), &period).await.unwrap();
        let view = service.work_view(&user(CLARA), None, None).await.unwrap();
        assert!(view.entries.iter().any(|e| e.hours == "2,5"));
    }

    #[tokio::test]
    async fn confirmed_entries_are_locked_and_changes_drop_confirmation() {
        let (service, _) = service();
        let confirmed = service.work_view(&user(CLARA), None, None).await.unwrap();
        let locked = confirmed.entries.iter().find(|e| e.confirmed).unwrap();
        assert!(!locked.can_edit);
        let id = locked.id.parse().unwrap();
        assert_eq!(
            service.delete_work_entry(&user(CLARA), id).await,
            Err(AppError::Forbidden)
        );
        let open = confirmed.entries.iter().find(|e| !e.confirmed).unwrap();
        let open_id = open.id.parse().unwrap();
        let (mut form, version) = service
            .work_entry_input(&user(CLARA), open_id)
            .await
            .unwrap();
        form.hours = "7".into();
        service
            .update_work_entry(&user(CLARA), open_id, version, &form)
            .await
            .unwrap();
        assert_eq!(
            service
                .update_work_entry(&user(CLARA), open_id, version, &form)
                .await,
            Err(AppError::Conflict)
        );
        service
            .delete_work_entry(&user(CLARA), open_id)
            .await
            .unwrap();
        let after = service.work_view(&user(CLARA), None, None).await.unwrap();
        assert!(after.entries.iter().all(|e| e.id != open.id));
        assert_eq!(
            service.delete_work_entry(&user(GRETA), open_id).await,
            Err(AppError::NotFound("Arbeitseinsatz".into()))
        );
    }

    #[tokio::test]
    async fn warden_confirms_entries_of_his_boat_but_never_his_own() {
        let (service, _) = service();
        let id = service
            .add_work_entry(&user(CLARA), &input("3"))
            .await
            .unwrap();
        let own = service
            .add_work_entry(&user(BEN), &input("1"))
            .await
            .unwrap();
        let open = service.open_confirmations(&user(BEN)).await.unwrap();
        assert!(
            open.iter()
                .any(|e| e.id == id.to_string() && e.member == "Clara Jensen")
        );
        assert!(
            open.iter().all(|e| e.id != own.to_string()),
            "eigene Einsätze nicht bestätigbar"
        );
        assert!(
            service
                .open_confirmations(&user(GRETA))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            service.confirm_work(&user(BEN), own).await,
            Err(AppError::Forbidden)
        );
        service.confirm_work(&user(BEN), id).await.unwrap();
        let view = service.work_view(&user(CLARA), None, None).await.unwrap();
        assert_eq!(view.credited, "23,5");
        assert_eq!(
            service.confirm_work(&user(GRETA), id).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn events_can_be_registered_until_full_and_never_credit_hours() {
        let (service, store) = service();
        let event = EventInput {
            title: "Herbstputz".into(),
            date: "2026-11-07".into(),
            start: "10:00".into(),
            end: "14:00".into(),
            max: "1".into(),
            ..Default::default()
        };
        assert_eq!(
            service.create_event(&user(GRETA), &event).await,
            Err(AppError::Forbidden)
        );
        let id = service.create_event(&user(FRANK), &event).await.unwrap();
        service.register_event(&user(ANNA), id).await.unwrap();
        let full = service.register_event(&user(GRETA), id).await;
        let Err(AppError::Rejected(e)) = full else {
            panic!()
        };
        assert!(e[0].message.contains("ausgebucht"));
        let again = service.register_event(&user(ANNA), id).await;
        assert!(matches!(again, Err(AppError::Rejected(_))));
        let view = service.events_view(&user(ANNA)).await.unwrap();
        let line = view.events.iter().find(|e| e.id == id.to_string()).unwrap();
        assert!(
            line.mine
                && line.full
                && line
                    .participants
                    .iter()
                    .map(|p| p.name.as_str())
                    .eq(["Anna Petersen"])
        );
        let credited_before = service
            .work_view(&user(ANNA), None, None)
            .await
            .unwrap()
            .credited;
        service.unregister_event(&user(ANNA), id).await.unwrap();
        service.register_event(&user(GRETA), id).await.unwrap();
        assert_eq!(
            service
                .work_view(&user(ANNA), None, None)
                .await
                .unwrap()
                .credited,
            credited_before
        );
        let (mut form, version) = service.event_input(&user(FRANK), id).await.unwrap();
        form.end = "09:00".into();
        let bad = service.update_event(&user(FRANK), id, version, &form).await;
        let Err(AppError::Rejected(e)) = bad else {
            panic!()
        };
        assert_eq!(e[0].field, "end");
        form.end = "15:00".into();
        service
            .update_event(&user(FRANK), id, version, &form)
            .await
            .unwrap();
        assert!(
            store
                .audit_entries()
                .iter()
                .any(|e| e.action == "event.edit")
        );
    }

    #[tokio::test]
    async fn tasks_are_assigned_and_updated_by_the_assignee() {
        let (service, _) = service();
        let task = TaskInput {
            title: "Fender flicken".into(),
            hours: "2,5".into(),
            ..Default::default()
        };
        assert_eq!(
            service.create_task(&user(GRETA), &task).await,
            Err(AppError::Forbidden)
        );
        let id = service.create_task(&user(BEN), &task).await.unwrap();
        service.assign_task(&user(BEN), id, GRETA).await.unwrap();
        let again = service.assign_task(&user(BEN), id, GRETA).await;
        assert!(matches!(again, Err(AppError::Rejected(_))));
        assert_eq!(
            service
                .set_task_status(&user(ANNA), id, TaskStatus::Done)
                .await,
            Ok(()),
            "Amtsträger darf"
        );
        service
            .set_task_status(&user(GRETA), id, TaskStatus::InProgress)
            .await
            .unwrap();
        let other = service
            .set_task_status(&user(CLARA), id, TaskStatus::Done)
            .await;
        assert_eq!(other, Ok(()), "Clara hat ein Amt");
        let view = service.tasks_view(&user(GRETA)).await.unwrap();
        let line = view.tasks.iter().find(|t| t.id == id.to_string()).unwrap();
        assert_eq!(line.status, "erledigt");
        assert!(line.can_update && line.assignees == ["Greta Lorenzen"]);
        let mut hans = user(vereinsakte_core::example::HANNES);
        hans.member = vereinsakte_core::example::HANNES;
        let denied = service.set_task_status(&hans, id, TaskStatus::Open).await;
        assert_eq!(
            denied,
            Err(AppError::Forbidden),
            "ohne Amt und ohne Zuständigkeit"
        );
    }

    #[tokio::test]
    async fn board_transfers_hours_and_compensates() {
        let (service, store) = service();
        let compensation = service
            .approve_compensation(&user(EVA), BEN, 2026, "5", None, "Studienbelastung")
            .await
            .unwrap();
        assert_eq!(compensation.to_string(), "75,00 €");
        let snapshot = store.snapshot();
        let claims = &snapshot.member(BEN).unwrap().account.claims;
        assert!(
            claims
                .iter()
                .any(|c| c.amount.to_string() == "75,00 €" && c.text.contains("Abgeltung"))
        );
        let twice = service
            .approve_compensation(&user(EVA), BEN, 2026, "1", None, "")
            .await;
        assert!(matches!(twice, Err(AppError::Rejected(_))));
        let same = service
            .transfer_hours(&user(EVA), CLARA, CLARA, 2026, "2", "")
            .await;
        assert!(matches!(same, Err(AppError::Rejected(_))));
        service
            .transfer_hours(&user(EVA), BEN, CLARA, 2026, "5", "Ausgleich")
            .await
            .unwrap();
        assert_eq!(
            service
                .transfer_hours(&user(GRETA), BEN, CLARA, 2026, "1", "")
                .await,
            Err(AppError::Forbidden)
        );
        let deficit = service
            .hours_overview(&user(EVA), None, true)
            .await
            .unwrap();
        assert!(deficit.rows.iter().all(|r| r.missing.is_some()));
        assert_eq!(
            service.hours_overview(&user(GRETA), None, false).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn deficit_list_shows_clara_before_compensation() {
        let (service, _) = service();
        let overview = service
            .hours_overview(&user(EVA), None, true)
            .await
            .unwrap();
        let clara = overview
            .rows
            .iter()
            .find(|r| r.name == "Jensen, Clara")
            .unwrap();
        assert_eq!(
            clara.missing.as_deref(),
            Some("es fehlen 4,5 Arbeitsstunden")
        );
    }
}

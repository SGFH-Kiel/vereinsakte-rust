//! Änderungsprotokoll: unveränderliche Einträge über jede schreibende Aktion.

use async_trait::async_trait;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;

/// Ein Eintrag im Änderungsprotokoll. Enthält nur Kennungen und Feldnamen,
/// keine Werte personenbezogener Felder und keine Anmeldedaten.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub at: Timestamp,
    /// Handelndes Mitglied.
    pub actor: MemberId,
    /// Aktionsart, z. B. `boat.block`.
    pub action: String,
    /// Art des betroffenen Objekts, z. B. `boat`.
    pub object_kind: String,
    /// Kennung des betroffenen Objekts.
    pub object_id: String,
    /// Betroffenes Mitglied, falls vorhanden.
    pub subject: Option<MemberId>,
    /// Kurze deutsche Beschreibung.
    pub summary: String,
}

/// Gespeicherter Eintrag mit laufender Nummer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: i64,
    pub entry: AuditEntry,
}

/// Filter und Seitenwahl für die Protokollansicht (neueste zuerst).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub from: Option<Timestamp>,
    /// Exklusive Obergrenze.
    pub to: Option<Timestamp>,
    pub actor: Option<MemberId>,
    pub action: Option<String>,
    /// Nur Einträge, in denen das Mitglied handelt oder betroffen ist.
    pub involving: Option<MemberId>,
    /// Nur Einträge zu diesem Objekt (Kennung).
    pub object_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: u32,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn audit(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>, AppError>;
}

/// Alle Aktionsarten des Protokolls mit deutscher Bezeichnung.
pub const ACTIONS: [(&str, &str); 49] = [
    ("boat.block", "Boot gesperrt"),
    ("boat.unblock", "Sperre aufgehoben"),
    ("boat.release", "Boot für Saison freigegeben"),
    ("boat.create", "Boot angelegt"),
    ("boat.edit", "Boot geändert"),
    ("boat.image_set", "Bootsbild gesetzt"),
    ("boat.image_removed", "Bootsbild entfernt"),
    ("boat.checkout", "Bootsabnahme"),
    ("boat.revoke", "Bootserlaubnis widerrufen"),
    ("claim.post", "Forderung angelegt"),
    ("claim.cancel", "Forderung storniert"),
    ("payment.post", "Einzahlung gebucht"),
    ("season.save", "Saison gespeichert"),
    ("fees.annual", "Jahresbeiträge angelegt"),
    ("fees.tour", "Tourgebühren angelegt"),
    ("event.create", "Arbeitstermin angelegt"),
    ("event.edit", "Arbeitstermin geändert"),
    ("event.register", "Zum Arbeitstermin angemeldet"),
    ("event.unregister", "Vom Arbeitstermin abgemeldet"),
    ("hours.transfer", "Stundenübertrag"),
    ("hours.compensation", "Abgeltung genehmigt"),
    ("member.create", "Mitglied aufgenommen"),
    ("member.edit", "Mitglied geändert"),
    ("member.contact", "Kontaktdaten geändert"),
    ("member.contact_block", "Sperrvermerk geändert"),
    ("member.leave", "Austritt erfasst"),
    ("member.expel", "Ausschluss erfasst"),
    ("member.license.add", "Schein oder Lizenz hinzugefügt"),
    ("member.license.remove", "Schein oder Lizenz entfernt"),
    ("office.appoint", "Amt vergeben"),
    ("office.resign", "Amt abgegeben"),
    ("reservation.create", "Reservierung angelegt"),
    ("reservation.edit", "Reservierung geändert"),
    ("reservation.cancel", "Reservierung storniert"),
    ("task.create", "Aufgabe angelegt"),
    ("task.assign", "Aufgabe zugewiesen"),
    ("task.status", "Aufgabenstatus geändert"),
    ("task.edit", "Aufgabe geändert"),
    ("task.unassign", "Zuständigkeit entzogen"),
    ("task.description_changed", "Aufgabenbeschreibung geändert"),
    ("task.comment_added", "Aufgabe kommentiert"),
    ("task.comment_edited", "Kommentar korrigiert"),
    ("task.comment_deleted", "Kommentar gelöscht"),
    ("task.attachment_added", "Anhang hinzugefügt"),
    ("task.attachment_removed", "Anhang entfernt"),
    ("work.add", "Arbeitseinsatz erfasst"),
    ("work.edit", "Arbeitseinsatz geändert"),
    ("work.delete", "Arbeitseinsatz gelöscht"),
    ("work.confirm", "Arbeitseinsatz bestätigt"),
];

/// Deutsche Bezeichnung einer Aktionsart (unbekannte Arten bleiben unverändert).
pub fn action_label(code: &str) -> String {
    ACTIONS
        .iter()
        .find(|(c, _)| *c == code)
        .map_or_else(|| code.to_string(), |(_, label)| (*label).to_string())
}

/// Eine Zeile der Protokollansicht.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRow {
    pub when: String,
    pub actor: crate::people::PersonRef,
    pub action: String,
    pub summary: String,
    pub subject: Option<crate::people::PersonRef>,
    pub object: String,
}

/// Anfrage an die Protokollansicht (Texte aus der Adresse).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditQuery {
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
    pub actor: Option<MemberId>,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditView {
    pub rows: Vec<AuditRow>,
    /// Sieht der Betrachter alle Einträge (Vorstand, Revisoren)?
    pub sees_all: bool,
    pub actions: Vec<(&'static str, &'static str)>,
    pub members: Vec<(u32, String)>,
}

impl crate::service::ClubService {
    /// Protokoll, neueste Einträge zuerst. Vorstand und Revisoren sehen alles,
    /// alle anderen nur Einträge, in denen sie handelnd oder betroffen sind.
    pub async fn audit_view(
        &self,
        user: &crate::service::CurrentUser,
        query: &AuditQuery,
    ) -> Result<AuditView, AppError> {
        use chrono::Datelike;
        use vereinsakte_core::rights::Capability;
        use vereinsakte_core::time::club_time;

        let principal = self.principal(user).await?;
        let sees_all = principal.can(&Capability::ReadAudit);
        let day = |d: chrono::NaiveDate| club_time(d.year(), d.month(), d.day(), 0, 0);
        let filter = AuditFilter {
            from: query.from.map(day),
            to: query.to.map(|d| day(d + chrono::Duration::days(1))),
            actor: query.actor.filter(|_| sees_all),
            action: Some(query.action.clone()).filter(|a| !a.is_empty()),
            involving: (!sees_all).then_some(user.member),
            ..AuditFilter::default()
        };
        let records = self.store.audit(&filter).await?;
        let members = self.store.active_members(principal.now).await?;
        let name = |m: MemberId| {
            members.iter().find(|x| x.id == m).map_or_else(
                || crate::people::PersonRef::text(format!("Mitglied {m}")),
                |x| crate::people::PersonRef::member(m, x.full_name()),
            )
        };
        Ok(AuditView {
            rows: records
                .into_iter()
                .map(|r| AuditRow {
                    when: r.entry.at.format("%d.%m.%Y %H:%M").to_string(),
                    actor: name(r.entry.actor),
                    action: action_label(&r.entry.action),
                    summary: r.entry.summary.clone(),
                    subject: r.entry.subject.map(name),
                    object: format!("{} {}", r.entry.object_kind, r.entry.object_id),
                })
                .collect(),
            sees_all,
            actions: ACTIONS.to_vec(),
            members: if sees_all {
                let mut list: Vec<_> = members
                    .iter()
                    .map(|m| (m.id.get(), m.full_name()))
                    .collect();
                list.sort_by(|a, b| a.1.cmp(&b.1));
                list
            } else {
                vec![]
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use vereinsakte_core::example::{ANNA, BEN, EVA, GRETA, HARALD_BLAUZAHN, LUKAS};

    use super::*;
    use crate::service::CurrentUser;
    use crate::service::tests::service;

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    #[tokio::test]
    async fn board_and_auditors_see_everything_members_only_their_own() {
        let (service, _) = service();
        service
            .block_boat(&user(BEN), HARALD_BLAUZAHN, "Mast defekt", None, None)
            .await
            .unwrap();
        service
            .add_license(
                &user(GRETA),
                GRETA,
                crate::members::LicenseKind::Sailing,
                "SKS",
                None,
            )
            .await
            .unwrap();
        let all = service
            .audit_view(&user(EVA), &AuditQuery::default())
            .await
            .unwrap();
        assert!(all.sees_all && all.rows.len() == 2);
        assert_eq!(all.rows[0].action, "Schein oder Lizenz hinzugefügt");
        let auditor = service
            .audit_view(&user(LUKAS), &AuditQuery::default())
            .await
            .unwrap();
        assert_eq!(auditor.rows.len(), 2);
        let own = service
            .audit_view(&user(GRETA), &AuditQuery::default())
            .await
            .unwrap();
        assert!(!own.sees_all && own.rows.len() == 1 && own.rows[0].actor.name == "Greta Lorenzen");
        let none = service
            .audit_view(&user(ANNA), &AuditQuery::default())
            .await
            .unwrap();
        assert!(none.sees_all, "Anna gehört zum Vorstand");
        let nothing = service
            .audit_view(
                &user(vereinsakte_core::example::HANNES),
                &AuditQuery::default(),
            )
            .await
            .unwrap();
        assert!(nothing.rows.is_empty());
    }

    #[tokio::test]
    async fn filters_by_period_actor_and_action_and_never_shows_values() {
        let (service, _) = service();
        service
            .block_boat(&user(BEN), HARALD_BLAUZAHN, "Mast defekt", None, None)
            .await
            .unwrap();
        service
            .post_payment(
                &user(vereinsakte_core::example::JONAS),
                ANNA,
                "12.50".parse().unwrap(),
                None,
                "geheim",
            )
            .await
            .unwrap();
        let by_action = service
            .audit_view(
                &user(EVA),
                &AuditQuery {
                    action: "payment.post".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(by_action.rows.len(), 1);
        assert_eq!(by_action.rows[0].action, "Einzahlung gebucht");
        assert!(
            !format!("{by_action:?}").contains("12.50")
                && !format!("{by_action:?}").contains("geheim")
        );
        let by_actor = service
            .audit_view(
                &user(EVA),
                &AuditQuery {
                    actor: Some(BEN),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(by_actor.rows.len(), 1);
        let today = NaiveDate::from_ymd_opt(2026, 10, 9);
        let in_period = service
            .audit_view(
                &user(EVA),
                &AuditQuery {
                    from: today,
                    to: today,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(in_period.rows.len(), 2);
        let past = service
            .audit_view(
                &user(EVA),
                &AuditQuery {
                    to: NaiveDate::from_ymd_opt(2026, 1, 1),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(past.rows.is_empty());
    }
}

#[cfg(test)]
mod sweep {
    //! Jede schreibende Aktion protokolliert genau einmal; abgelehnte Aktionen gar nicht.

    use std::collections::BTreeSet;

    use chrono::NaiveDate;
    use vereinsakte_core::example::{
        ANNA, BEN, CLARA, DAVID, EVA, FILOU, FRANK, GRETA, HARALD_BLAUZAHN, JONAS, MALTE,
    };
    use vereinsakte_core::office::Role;
    use vereinsakte_core::task::TaskStatus;

    use super::*;
    use crate::boats::BoatInput;
    use crate::members::{ContactInput, LicenseKind, MemberInput};
    use crate::reservations::ReservationInput;
    use crate::service::{CurrentUser, tests::service};
    use crate::work::{EntryInput, EventInput, TaskInput};

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    #[tokio::test]
    async fn every_command_writes_exactly_one_entry_and_failures_none() {
        let (service, store) = service();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let day = NaiveDate::from_ymd_opt(2026, 10, 31);

        macro_rules! logged {
            ($code:expr, $call:expr) => {{
                let before = store.audit_entries().len();
                let result = $call.await;
                assert!(result.is_ok(), "{}: {:?}", $code, result.as_ref().err());
                let log = store.audit_entries();
                assert_eq!(
                    log.len(),
                    before + 1,
                    "{} schreibt genau einen Eintrag",
                    $code
                );
                assert_eq!(log.last().unwrap().action, $code);
                seen.insert($code.to_string());
                result.unwrap()
            }};
        }
        macro_rules! refused {
            ($call:expr) => {{
                let before = store.audit_entries().len();
                let snapshot = store.snapshot();
                assert!($call.await.is_err());
                assert_eq!(
                    store.audit_entries().len(),
                    before,
                    "kein Eintrag bei Ablehnung"
                );
                assert_eq!(
                    store.snapshot(),
                    snapshot,
                    "keine Teiländerung bei Ablehnung"
                );
            }};
        }

        // Mitglieder
        let form = service.member_form(&user(ANNA), ANNA).await.unwrap();
        let mut contact = form.contact.clone();
        contact.city = "Eckernförde".into();
        logged!(
            "member.contact",
            service.update_contact(&user(ANNA), ANNA, form.version, &contact)
        );
        logged!(
            "member.contact_block",
            service.set_contact_blocked(&user(ANNA), ANNA, true)
        );
        let input = |number| MemberInput {
            number,
            first_name: "Nora".into(),
            last_name: "Neu".into(),
            contact: ContactInput {
                street: "Hafenstraße 1".into(),
                postal_code: "24103".into(),
                city: "Kiel".into(),
                ..Default::default()
            },
            login_email: String::new(),
            membership_type: vereinsakte_core::member::MembershipType::Regular,
            board_consent: false,
            address_valid: true,
            child_of: None,
            joined: None,
        };
        let nora = logged!(
            "member.create",
            service.create_member(&user(EVA), &input(None))
        );
        let nora_form = service.member_form(&user(EVA), nora).await.unwrap();
        let mut changed = input(None);
        changed.first_name = "Norah".into();
        logged!(
            "member.edit",
            service.update_member(&user(EVA), nora, nora_form.version, &changed)
        );
        let nora_form = service.member_form(&user(EVA), nora).await.unwrap();
        logged!(
            "member.leave",
            service.end_membership(&user(EVA), nora, nora_form.version, day.unwrap(), false)
        );
        let malte = service.member_form(&user(EVA), MALTE).await.unwrap();
        logged!(
            "member.expel",
            service.end_membership(&user(EVA), MALTE, malte.version, day.unwrap(), true)
        );
        logged!(
            "member.license.add",
            service.add_license(&user(GRETA), GRETA, LicenseKind::Sailing, "SKS", None)
        );
        logged!(
            "member.license.remove",
            service.remove_license(&user(GRETA), GRETA, LicenseKind::Sailing, 0)
        );

        // Ämter
        logged!(
            "office.appoint",
            service.appoint(&user(FRANK), GRETA, Role::Boatwarden(FILOU), None)
        );
        let office = store
            .snapshot()
            .offices
            .iter()
            .find(|o| o.member == GRETA && o.role == Role::Boatwarden(FILOU))
            .unwrap()
            .id;
        logged!("office.resign", service.resign(&user(GRETA), office, None));

        // Boote
        let boat = BoatInput {
            name: "Neptun".into(),
            dinghy: true,
            ..Default::default()
        };
        let neptun = logged!("boat.create", service.create_boat(&user(FRANK), &boat));
        let boat_form = service.boat_form(&user(FRANK), neptun).await.unwrap();
        let mut edited = boat_form.input.clone();
        edited.class = "Pirat".into();
        logged!(
            "boat.edit",
            service.update_boat(&user(FRANK), neptun, boat_form.version, &edited)
        );
        logged!(
            "boat.block",
            service.block_boat(&user(BEN), HARALD_BLAUZAHN, "Mast", None, None)
        );
        logged!(
            "boat.unblock",
            service.lift_block(&user(BEN), HARALD_BLAUZAHN)
        );
        logged!(
            "boat.release",
            service.release_boat(&user(FRANK), FILOU, Some(2027))
        );
        logged!(
            "boat.checkout",
            service.record_checkout(&user(DAVID), FILOU, GRETA, true, "")
        );
        logged!(
            "boat.revoke",
            service.revoke_permit(&user(EVA), FILOU, GRETA, "Unfall")
        );

        // Arbeit
        let entry = |hours: &str| EntryInput {
            date: "2026-10-01".into(),
            hours: hours.into(),
            subject: HARALD_BLAUZAHN.to_string(),
            ..Default::default()
        };
        let first = logged!(
            "work.add",
            service.add_work_entry(&user(CLARA), &entry("3"))
        );
        let (mut form, version) = service.work_entry_input(&user(CLARA), first).await.unwrap();
        form.hours = "4".into();
        logged!(
            "work.edit",
            service.update_work_entry(&user(CLARA), first, version, &form)
        );
        logged!("work.confirm", service.confirm_work(&user(BEN), first));
        let second = service
            .add_work_entry(&user(CLARA), &entry("1"))
            .await
            .unwrap();
        logged!(
            "work.delete",
            service.delete_work_entry(&user(CLARA), second)
        );
        let event = EventInput {
            title: "Herbstputz".into(),
            date: "2026-11-07".into(),
            start: "10:00".into(),
            end: "14:00".into(),
            ..Default::default()
        };
        let event_id = logged!("event.create", service.create_event(&user(FRANK), &event));
        let (mut event_form, event_version) =
            service.event_input(&user(FRANK), event_id).await.unwrap();
        event_form.location = "Hafen".into();
        logged!(
            "event.edit",
            service.update_event(&user(FRANK), event_id, event_version, &event_form)
        );
        logged!(
            "event.register",
            service.register_event(&user(ANNA), event_id)
        );
        logged!(
            "event.unregister",
            service.unregister_event(&user(ANNA), event_id)
        );
        let task = logged!(
            "task.create",
            service.create_task(
                &user(BEN),
                &TaskInput {
                    title: "Fender".into(),
                    ..Default::default()
                }
            )
        );
        logged!("task.assign", service.assign_task(&user(BEN), task, GRETA));
        logged!(
            "task.status",
            service.set_task_status(&user(GRETA), task, TaskStatus::Done)
        );
        let task_form = TaskInput {
            title: "Fender neu".into(),
            ..Default::default()
        };
        logged!(
            "boat.image_set",
            service.set_boat_image(&user(FRANK), FILOU, b"GIF89a....".to_vec(), "")
        );
        logged!(
            "boat.image_removed",
            service.remove_boat_image(&user(FRANK), FILOU)
        );
        logged!(
            "task.edit",
            service.edit_task(&user(BEN), task, &task_form, None)
        );
        logged!(
            "task.unassign",
            service.unassign_task(&user(BEN), task, GRETA)
        );
        logged!(
            "task.description_changed",
            service.set_task_description(&user(BEN), task, "**fett**", None)
        );
        logged!(
            "task.comment_added",
            service.add_comment(&user(ANNA), task, "Segeltuch ist bestellt")
        );
        let comment = service.task_page(&user(ANNA), task).await.unwrap().comments[0]
            .id
            .parse()
            .unwrap();
        logged!(
            "task.comment_edited",
            service.edit_comment(&user(ANNA), comment, "Segeltuch ist da")
        );
        logged!(
            "task.comment_deleted",
            service.delete_comment(&user(ANNA), comment)
        );
        logged!(
            "task.attachment_added",
            service.add_attachment(&user(ANNA), task, "a.pdf", b"%PDF-1.4 x".to_vec(), "")
        );
        let attachment = service
            .task_page(&user(ANNA), task)
            .await
            .unwrap()
            .attachments[0]
            .id
            .parse()
            .unwrap();
        logged!(
            "task.attachment_removed",
            service.delete_attachment(&user(ANNA), attachment)
        );
        logged!(
            "hours.transfer",
            service.transfer_hours(&user(EVA), ANNA, CLARA, 2026, "2", "")
        );
        logged!(
            "hours.compensation",
            service.approve_compensation(&user(EVA), BEN, 2026, "5", None, "")
        );

        // Reservierungen
        let reservation = ReservationInput {
            boat: FILOU.to_string(),
            area: "home".into(),
            from_date: "2026-10-17".into(),
            from_time: "10:00".into(),
            to_date: "2026-10-19".into(),
            to_time: "16:00".into(),
            ..Default::default()
        };
        let res = logged!(
            "reservation.create",
            service.create_reservation(&user(DAVID), &reservation)
        );
        let (mut res_form, res_version) =
            service.reservation_input(&user(DAVID), res).await.unwrap();
        res_form.remark = "Wochenende".into();
        logged!(
            "reservation.edit",
            service.update_reservation(&user(DAVID), res, res_version, &res_form)
        );
        let billed = logged!("fees.tour", service.bill_tour(&user(JONAS), res));
        assert!(billed > 0);
        logged!(
            "reservation.cancel",
            service.cancel_reservation(&user(DAVID), res)
        );

        // Finanzen
        logged!(
            "payment.post",
            service.post_payment(&user(JONAS), ANNA, "10".parse().unwrap(), None, "")
        );
        logged!(
            "claim.post",
            service.post_claim(&user(JONAS), ANNA, "20".parse().unwrap(), "Pfand", None)
        );
        let claim = store
            .snapshot()
            .member(ANNA)
            .unwrap()
            .account
            .claims
            .iter()
            .find(|c| c.text == "Pfand")
            .unwrap()
            .id;
        logged!(
            "claim.cancel",
            service.cancel_claim(&user(JONAS), ANNA, claim, "Irrtum")
        );
        let season = service
            .seasons_view(&user(JONAS), Some(2028))
            .await
            .unwrap();
        let mut fields = std::collections::HashMap::new();
        fields.insert("hours_quota".to_string(), season.form.hours_quota.clone());
        fields.insert(
            "compensation_rate".to_string(),
            season.form.compensation_rate.clone(),
        );
        for f in season
            .form
            .annual
            .iter()
            .chain(&season.form.admission)
            .chain(&season.form.tour)
        {
            fields.insert(f.key.clone(), f.value.clone());
        }
        logged!(
            "season.save",
            service.save_season(&user(JONAS), 2028, &fields)
        );
        let created = logged!(
            "fees.annual",
            service.create_annual_fees(&user(JONAS), 2028)
        );
        assert!(created > 0);

        // Vollständigkeit: genau die dokumentierten Aktionsarten.
        let documented: BTreeSet<String> = ACTIONS
            .iter()
            .map(|(code, _)| (*code).to_string())
            .collect();
        assert_eq!(seen, documented);

        // Ablehnungen hinterlassen weder Eintrag noch Teiländerung.
        refused!(service.block_boat(&user(GRETA), FILOU, "x", None, None));
        refused!(service.post_payment(&user(JONAS), ANNA, "-1".parse().unwrap(), None, ""));
        refused!(service.update_contact(&user(ANNA), ANNA, 999, &contact));
        refused!(service.set_contact_blocked(&user(GRETA), ANNA, true));
        refused!(service.create_member(&user(GRETA), &input(None)));
        refused!(service.approve_compensation(&user(EVA), BEN, 2026, "5", None, ""));
    }
}

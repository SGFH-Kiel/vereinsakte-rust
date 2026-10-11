//! Schreiben der Vereinsakte: je Aggregat Elternzeile per Upsert, Kindzeilen
//! ersetzen; beim Speichern werden nur geänderte Aggregate geschrieben.

use std::collections::HashMap;

use sqlx::SqliteConnection;
use vereinsakte_core::account::ClaimKind;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::boat_permit::{BoatCheckout, Revocation};
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::fees::Season;
use vereinsakte_core::member::Member;
use vereinsakte_core::office::{Office, Role};
use vereinsakte_core::requirement::{Qualification, Requirement};
use vereinsakte_core::reservation::Reservation;
use vereinsakte_core::sailing_permit::AnnualRecord;
use vereinsakte_core::task::Task;
use vereinsakte_core::work_event::WorkEvent;
use vereinsakte_core::work_hours::{
    HourTransfer, WorkEntry, WorkExtent, WorkReference, WorkSubject,
};

use crate::db::StorageError;
use crate::values::*;

/// Ein unabhängig speicherbares Fachobjekt samt Kindzeilen.
pub(crate) trait Aggregate: PartialEq {
    const TABLE: &'static str;

    /// Schlüsselspalten der Elternzeile.
    fn key(&self) -> Vec<(&'static str, Val)>;

    /// Schlüssel als Text für den Abgleich.
    fn key_text(&self) -> String;

    /// Schreibt Elternzeile (Upsert) und ersetzt alle Kindzeilen.
    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError>;
}

/// Gleicht eine Sammlung ab: geänderte und neue Aggregate schreiben, bei nur
/// verschobener Position die Position anpassen, entfernte löschen.
pub(crate) async fn sync<T: Aggregate>(
    conn: &mut SqliteConnection,
    old: &[T],
    new: &[T],
) -> Result<(), StorageError> {
    let previous: HashMap<String, (usize, &T)> = old
        .iter()
        .enumerate()
        .map(|(i, item)| (item.key_text(), (i, item)))
        .collect();
    for (position, item) in new.iter().enumerate() {
        match previous.get(&item.key_text()) {
            Some((_, before)) if *before != item => item.write(conn, position as i64).await?,
            Some((old_position, _)) if *old_position != position => {
                set_position(conn, T::TABLE, item.key(), position as i64).await?;
            }
            Some(_) => {}
            None => item.write(conn, position as i64).await?,
        }
    }
    let current: std::collections::HashSet<String> = new.iter().map(Aggregate::key_text).collect();
    for item in old
        .iter()
        .filter(|item| !current.contains(&item.key_text()))
    {
        delete_where(conn, T::TABLE, item.key()).await?;
    }
    Ok(())
}

/// Schreibt alle Unterschiede zwischen `old` und `new`.
pub(crate) async fn write_diff(
    conn: &mut SqliteConnection,
    old: &ClubRecord,
    new: &ClubRecord,
) -> Result<(), StorageError> {
    sync(conn, &old.members, &new.members).await?;
    sync(conn, &old.boats, &new.boats).await?;
    sync(conn, &old.offices, &new.offices).await?;
    sync(conn, &old.boat_checkouts, &new.boat_checkouts).await?;
    sync(conn, &old.revocations, &new.revocations).await?;
    sync(conn, &old.work_events, &new.work_events).await?;
    sync(conn, &old.tasks, &new.tasks).await?;
    sync(conn, &old.work_entries, &new.work_entries).await?;
    sync(conn, &old.hour_transfers, &new.hour_transfers).await?;
    sync(conn, &old.seasons, &new.seasons).await?;
    sync(conn, &old.annual_records, &new.annual_records).await?;
    sync(conn, &old.reservations, &new.reservations).await?;
    Ok(())
}

impl Aggregate for Member {
    const TABLE: &'static str = "members";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("id", member(self.id))]
    }

    fn key_text(&self) -> String {
        self.id.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        let m = &self.membership;
        upsert(
            conn,
            "members",
            &["id"],
            vec![
                ("id", member(self.id)),
                ("position", int(position)),
                ("last_name", text(&self.last_name)),
                ("first_name", text(&self.first_name)),
                ("street", text(&self.address.street)),
                ("postal_code", text(&self.address.postal_code)),
                ("city", text(&self.address.city)),
                ("phone_landline", opt_text(self.phone.landline.as_ref())),
                ("phone_business", opt_text(self.phone.business.as_ref())),
                ("phone_mobile", opt_text(self.phone.mobile.as_ref())),
                ("email", opt_text(self.email.as_ref())),
                ("login_email", opt_text(self.login_email.as_ref())),
                ("address_valid", boolean(self.address_valid)),
                ("membership_type", choice(&self.membership_type)),
                ("child_of", opt_member(self.child_of)),
                ("board_consent", boolean(self.board_consent)),
                ("joined_at", ts(m.joined_at)),
                ("provisional_until", opt_ts(m.provisional_until)),
                ("limited_until", opt_ts(m.limited_until)),
                ("left_at", opt_ts(m.left_at)),
                ("expelled_at", opt_ts(m.expelled_at)),
            ],
        )
        .await?;
        for table in [
            "member_sailing_licenses",
            "member_radio_licenses",
            "claims",
            "payments",
        ] {
            delete_where(conn, table, vec![("member_id", member(self.id))]).await?;
        }
        for (i, license) in self.sailing_licenses.iter().enumerate() {
            insert(
                conn,
                "member_sailing_licenses",
                vec![
                    ("member_id", member(self.id)),
                    ("position", int(i as i64)),
                    ("kind", text(&license.kind)),
                    ("acquired_at", opt_ts(license.acquired_at)),
                ],
            )
            .await?;
        }
        for (i, license) in self.radio_licenses.iter().enumerate() {
            insert(
                conn,
                "member_radio_licenses",
                vec![
                    ("member_id", member(self.id)),
                    ("position", int(i as i64)),
                    ("kind", text(&license.kind)),
                    ("acquired_at", opt_ts(license.acquired_at)),
                ],
            )
            .await?;
        }
        for (i, claim) in self.account.claims.iter().enumerate() {
            let (kind, year, reservation, guest) = match &claim.kind {
                ClaimKind::AnnualFee { year } => ("annual_fee", Some(*year), None, None),
                ClaimKind::AdmissionFee => ("admission_fee", None, None, None),
                ClaimKind::TourFee { reservation, guest } => {
                    ("tour_fee", None, Some(*reservation), guest.clone())
                }
                ClaimKind::WorkCompensation { year } => {
                    ("work_compensation", Some(*year), None, None)
                }
                ClaimKind::Other => ("other", None, None, None),
            };
            insert(
                conn,
                "claims",
                vec![
                    ("id", text(claim.id)),
                    ("member_id", member(self.id)),
                    ("position", int(i as i64)),
                    ("kind", text(kind)),
                    ("year", opt_int(year)),
                    ("reservation_id", opt_text(reservation)),
                    ("guest", opt_text(guest)),
                    ("amount_cents", money(claim.amount)),
                    ("text", text(&claim.text)),
                    ("created_at", ts(claim.created_at)),
                    ("due_at", opt_ts(claim.due_at)),
                    (
                        "cancelled_at",
                        opt_ts(claim.cancelled.as_ref().map(|c| c.at)),
                    ),
                    (
                        "cancel_reason",
                        opt_text(claim.cancelled.as_ref().map(|c| &c.reason)),
                    ),
                ],
            )
            .await?;
        }
        for (i, payment) in self.account.payments.iter().enumerate() {
            insert(
                conn,
                "payments",
                vec![
                    ("id", text(payment.id)),
                    ("member_id", member(self.id)),
                    ("position", int(i as i64)),
                    ("amount_cents", money(payment.amount)),
                    ("at", ts(payment.at)),
                    ("text", opt_text(payment.text.as_ref())),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

/// Schreibt die Gruppen einer Anforderung; `area` `None` = Skipper-Anforderung.
async fn write_requirement(
    conn: &mut SqliteConnection,
    boat: &Boat,
    area: Option<Val>,
    requirement: &Requirement,
) -> Result<(), StorageError> {
    for (i, group) in requirement.0.iter().enumerate() {
        let group_id = insert(
            conn,
            "requirement_groups",
            vec![
                ("boat_id", text(boat.id)),
                ("area", area.clone().unwrap_or(Val::Text(None))),
                ("position", int(i as i64)),
            ],
        )
        .await?;
        for (j, qualification) in group.0.iter().enumerate() {
            let (kind, license, boat_ref, boat_type) = match qualification {
                Qualification::SailingLicense(kind) => {
                    ("sailing_license", Some(kind.to_string()), None, None)
                }
                Qualification::RadioLicense(kind) => {
                    ("radio_license", Some(kind.to_string()), None, None)
                }
                Qualification::BoatPermit(other) => ("boat_permit", None, Some(*other), None),
                Qualification::BoatPermitForType(boat_type) => {
                    ("boat_permit_for_type", None, None, Some(choice(boat_type)))
                }
            };
            insert(
                conn,
                "requirement_qualifications",
                vec![
                    ("group_id", int(group_id)),
                    ("position", int(j as i64)),
                    ("kind", text(kind)),
                    ("license", opt_text(license)),
                    ("boat_ref", opt_text(boat_ref)),
                    ("boat_type", boat_type.unwrap_or(Val::Text(None))),
                ],
            )
            .await?;
        }
    }
    Ok(())
}

impl Aggregate for Boat {
    const TABLE: &'static str = "boats";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("id", text(self.id))]
    }

    fn key_text(&self) -> String {
        self.id.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        upsert(
            conn,
            "boats",
            &["id"],
            vec![
                ("id", text(self.id)),
                ("position", int(position)),
                ("name", text(&self.name)),
                ("boat_type", choice(&self.boat_type)),
                ("class", opt_text(self.class.as_ref())),
                ("year_built", opt_int(self.year_built)),
                ("length_m", opt_real(self.length_m)),
                ("beam_m", opt_real(self.beam_m)),
                ("draft_m", opt_real(self.draft_m)),
                ("sail_area_m2", opt_real(self.sail_area_m2)),
                ("berths", opt_int(self.berths)),
                ("engine", opt_text(self.engine.as_ref())),
                ("mooring", opt_text(self.mooring.as_ref())),
            ],
        )
        .await?;
        for table in [
            "requirement_groups",
            "crew_rules",
            "boat_releases",
            "boat_blocks",
        ] {
            delete_where(conn, table, vec![("boat_id", text(self.id))]).await?;
        }
        write_requirement(conn, self, None, &self.skipper_requirement).await?;
        for (i, rule) in self.crew_rules.iter().enumerate() {
            insert(
                conn,
                "crew_rules",
                vec![
                    ("boat_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("area", choice(&rule.area)),
                    ("min_members", int(rule.min_members)),
                    ("min_persons", int(rule.min_persons)),
                    ("co_skipper_required", boolean(rule.co_skipper_required)),
                ],
            )
            .await?;
            write_requirement(
                conn,
                self,
                Some(choice(&rule.area)),
                &rule.co_skipper_requirement,
            )
            .await?;
        }
        for (i, release) in self.releases.iter().enumerate() {
            insert(
                conn,
                "boat_releases",
                vec![
                    ("boat_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("season", int(release.season)),
                    ("by_member", member(release.by)),
                    ("at", ts(release.at)),
                ],
            )
            .await?;
        }
        for (i, block) in self.blocks.iter().enumerate() {
            insert(
                conn,
                "boat_blocks",
                vec![
                    ("boat_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("start_at", ts(block.start)),
                    ("end_at", opt_ts(block.end)),
                    ("reason", text(&block.reason)),
                    ("by_member", member(block.by)),
                    ("lifted_by", opt_member(block.lifted_by)),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

/// Rolle als (Wert der Spalte `role`, Boot).
pub(crate) fn role_parts(role: Role) -> (&'static str, Option<vereinsakte_core::ids::BoatId>) {
    match role {
        Role::Chair => ("chair", None),
        Role::ViceChair => ("vice_chair", None),
        Role::Treasurer => ("treasurer", None),
        Role::Secretary => ("secretary", None),
        Role::HeadBoatwarden => ("head_boatwarden", None),
        Role::Auditor => ("auditor", None),
        Role::Boatwarden(boat) => ("boatwarden", Some(boat)),
        Role::Examiner(boat) => ("examiner", Some(boat)),
    }
}

macro_rules! single_row_aggregate {
    ($type:ty, $table:literal, |$this:ident| $key:expr, |$s:ident, $pos:ident| $columns:expr) => {
        impl Aggregate for $type {
            const TABLE: &'static str = $table;

            fn key(&self) -> Vec<(&'static str, Val)> {
                let $this = self;
                vec![("id", text($key))]
            }

            fn key_text(&self) -> String {
                let $this = self;
                $key.to_string()
            }

            async fn write(
                &self,
                conn: &mut SqliteConnection,
                $pos: i64,
            ) -> Result<(), StorageError> {
                let $s = self;
                upsert(conn, $table, &["id"], $columns).await
            }
        }
    };
}

single_row_aggregate!(Office, "offices", |o| o.id, |o, position| {
    let (role, boat) = role_parts(o.role);
    vec![
        ("id", text(o.id)),
        ("position", int(position)),
        ("role", text(role)),
        ("role_boat_id", opt_text(boat)),
        ("member_id", member(o.member)),
        ("start_at", ts(o.start)),
        ("end_at", opt_ts(o.end)),
    ]
});

single_row_aggregate!(
    BoatCheckout,
    "boat_checkouts",
    |c| c.id,
    |c, position| vec![
        ("id", text(c.id)),
        ("position", int(position)),
        ("member_id", member(c.member)),
        ("boat_id", text(c.boat)),
        ("at", ts(c.at)),
        ("result", choice(&c.result)),
        ("examiner_id", member(c.examiner)),
        ("remark", opt_text(c.remark.as_ref())),
    ]
);

single_row_aggregate!(Revocation, "revocations", |r| r.id, |r, position| vec![
    ("id", text(r.id)),
    ("position", int(position)),
    ("member_id", member(r.member)),
    ("boat_id", text(r.boat)),
    ("at", ts(r.at)),
    ("reason", text(&r.reason)),
    ("by_member", member(r.by)),
]);

single_row_aggregate!(WorkEntry, "work_entries", |e| e.id, |e, position| {
    let (date, hours, start, end) = match &e.extent {
        WorkExtent::Amount { date, hours } => (Some(*date), Some(*hours), None, None),
        WorkExtent::Period { start, end } => (None, None, Some(*start), Some(*end)),
    };
    let subject = match e.subject {
        WorkSubject::Boat(boat) => Some(boat),
        WorkSubject::Club => None,
    };
    let (event, task) = match e.reference {
        Some(WorkReference::WorkEvent(id)) => (Some(id), None),
        Some(WorkReference::Task(id)) => (None, Some(id)),
        None => (None, None),
    };
    vec![
        ("id", text(e.id)),
        ("position", int(position)),
        ("member_id", member(e.member)),
        ("date_at", opt_ts(date)),
        ("hours", opt_real(hours)),
        ("start_at", opt_ts(start)),
        ("end_at", opt_ts(end)),
        ("subject_boat_id", opt_text(subject)),
        ("work_event_id", opt_text(event)),
        ("task_id", opt_text(task)),
        ("description", opt_text(e.description.as_ref())),
        ("confirmed_by", opt_member(e.confirmation.map(|c| c.by))),
        ("confirmed_at", opt_ts(e.confirmation.map(|c| c.at))),
    ]
});

single_row_aggregate!(
    HourTransfer,
    "hour_transfers",
    |t| t.id,
    |t, position| vec![
        ("id", text(t.id)),
        ("position", int(position)),
        ("year", int(t.year)),
        ("from_member", member(t.from)),
        ("to_member", member(t.to)),
        ("hours", real(t.hours)),
        ("description", opt_text(t.description.as_ref())),
    ]
);

impl Aggregate for WorkEvent {
    const TABLE: &'static str = "work_events";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("id", text(self.id))]
    }

    fn key_text(&self) -> String {
        self.id.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        upsert(
            conn,
            "work_events",
            &["id"],
            vec![
                ("id", text(self.id)),
                ("position", int(position)),
                ("title", text(&self.title)),
                ("start_at", ts(self.start)),
                ("end_at", ts(self.end)),
                ("location", opt_text(self.location.as_ref())),
                ("description", opt_text(self.description.as_ref())),
                ("boat_id", opt_text(self.boat)),
                ("max_participants", opt_int(self.max_participants)),
            ],
        )
        .await?;
        delete_where(
            conn,
            "work_event_registrations",
            vec![("event_id", text(self.id))],
        )
        .await?;
        for (i, registered) in self.registrations.iter().enumerate() {
            insert(
                conn,
                "work_event_registrations",
                vec![
                    ("event_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("member_id", member(*registered)),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

impl Aggregate for Task {
    const TABLE: &'static str = "tasks";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("id", text(self.id))]
    }

    fn key_text(&self) -> String {
        self.id.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        upsert(
            conn,
            "tasks",
            &["id"],
            vec![
                ("id", text(self.id)),
                ("position", int(position)),
                ("title", text(&self.title)),
                ("description", opt_text(self.description.as_ref())),
                ("boat_id", opt_text(self.boat)),
                ("estimated_hours", opt_real(self.estimated_hours)),
                ("due_at", opt_ts(self.due_at)),
                ("status", choice(&self.status)),
                ("done_at", opt_ts(self.done_at)),
            ],
        )
        .await?;
        delete_where(conn, "task_assignees", vec![("task_id", text(self.id))]).await?;
        for (i, assignee) in self.assignees.iter().enumerate() {
            insert(
                conn,
                "task_assignees",
                vec![
                    ("task_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("member_id", member(*assignee)),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

impl Aggregate for Season {
    const TABLE: &'static str = "seasons";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("year", int(self.year))]
    }

    fn key_text(&self) -> String {
        self.year.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        upsert(
            conn,
            "seasons",
            &["year"],
            vec![
                ("year", int(self.year)),
                ("position", int(position)),
                ("hours_quota", opt_real(self.hours_quota)),
                ("compensation_rate_cents", opt_money(self.compensation_rate)),
                ("has_fee_schedule", boolean(self.fees.is_some())),
            ],
        )
        .await?;
        for table in ["season_type_rates", "season_tour_rates"] {
            delete_where(conn, table, vec![("year", int(self.year))]).await?;
        }
        let Some(fees) = &self.fees else {
            return Ok(());
        };
        for (schedule, rates) in [
            ("annual", &fees.annual_fee),
            ("admission", &fees.admission_fee),
        ] {
            for (i, rate) in rates.iter().enumerate() {
                insert(
                    conn,
                    "season_type_rates",
                    vec![
                        ("year", int(self.year)),
                        ("schedule", text(schedule)),
                        ("position", int(i as i64)),
                        ("membership_type", choice(&rate.membership_type)),
                        ("amount_cents", money(rate.amount)),
                    ],
                )
                .await?;
            }
        }
        for (i, rate) in fees.tour.iter().enumerate() {
            insert(
                conn,
                "season_tour_rates",
                vec![
                    ("year", int(self.year)),
                    ("position", int(i as i64)),
                    ("tariff_group", choice(&rate.tariff_group)),
                    ("day_cents", money(rate.day)),
                    ("week_cents", money(rate.week)),
                    ("first_week_cents", opt_money(rate.first_week)),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

impl Aggregate for AnnualRecord {
    const TABLE: &'static str = "annual_records";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("member_id", member(self.member)), ("year", int(self.year))]
    }

    fn key_text(&self) -> String {
        format!("{}:{}", self.member, self.year)
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        let reduction = self.reduced_quota.as_ref();
        let compensation = self.compensation.as_ref();
        upsert(
            conn,
            "annual_records",
            &["member_id", "year"],
            vec![
                ("member_id", member(self.member)),
                ("year", int(self.year)),
                ("position", int(position)),
                ("reduced_hours", opt_real(reduction.map(|r| r.hours))),
                ("reduction_reason", opt_text(reduction.map(|r| &r.reason))),
                (
                    "compensation_hours",
                    opt_real(compensation.map(|c| c.hours)),
                ),
                (
                    "compensation_cents",
                    opt_money(compensation.and_then(|c| c.amount)),
                ),
                (
                    "compensation_approved_at",
                    opt_ts(compensation.map(|c| c.approved_at)),
                ),
                (
                    "compensation_reason",
                    opt_text(compensation.and_then(|c| c.reason.as_ref())),
                ),
            ],
        )
        .await
    }
}

impl Aggregate for Reservation {
    const TABLE: &'static str = "reservations";

    fn key(&self) -> Vec<(&'static str, Val)> {
        vec![("id", text(self.id))]
    }

    fn key_text(&self) -> String {
        self.id.to_string()
    }

    async fn write(&self, conn: &mut SqliteConnection, position: i64) -> Result<(), StorageError> {
        upsert(
            conn,
            "reservations",
            &["id"],
            vec![
                ("id", text(self.id)),
                ("position", int(position)),
                ("boat_id", text(self.boat)),
                ("start_at", ts(self.start)),
                ("end_at", ts(self.end)),
                ("area", choice(&self.area)),
                ("tour", boolean(self.tour)),
                ("skipper_id", member(self.skipper)),
                ("co_skipper_id", opt_member(self.co_skipper)),
                ("remark", opt_text(self.remark.as_ref())),
            ],
        )
        .await?;
        for table in ["reservation_crew", "reservation_guests"] {
            delete_where(conn, table, vec![("reservation_id", text(self.id))]).await?;
        }
        for (i, seat) in self.crew.iter().enumerate() {
            insert(
                conn,
                "reservation_crew",
                vec![
                    ("reservation_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("member_id", member(seat.member)),
                    ("from_at", opt_ts(seat.from)),
                    ("until_at", opt_ts(seat.until)),
                ],
            )
            .await?;
        }
        for (i, guest) in self.guests.iter().enumerate() {
            insert(
                conn,
                "reservation_guests",
                vec![
                    ("reservation_id", text(self.id)),
                    ("position", int(i as i64)),
                    ("name", text(&guest.name)),
                    ("phone", opt_text(guest.phone.as_ref())),
                    ("invited_by", opt_member(guest.invited_by)),
                    ("from_at", opt_ts(guest.from)),
                    ("until_at", opt_ts(guest.until)),
                ],
            )
            .await?;
        }
        Ok(())
    }
}

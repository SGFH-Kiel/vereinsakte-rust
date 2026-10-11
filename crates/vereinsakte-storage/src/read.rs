//! Laden der Vereinsakte aus dem relationalen Schema.

use std::collections::HashMap;
use std::hash::Hash;

use sqlx::SqliteConnection;
use sqlx::sqlite::SqliteRow;
use vereinsakte_core::account::{Cancellation, Claim, ClaimKind, MemberAccount, Payment};
use vereinsakte_core::boat::{Block, Boat, CrewRule, Release};
use vereinsakte_core::boat_permit::{BoatCheckout, Revocation};
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::fees::{FeeSchedule, Season, TourRate, TypeRate};
use vereinsakte_core::ids::*;
use vereinsakte_core::member::{
    Address, Member, Membership, PhoneNumbers, RadioLicense, RadioLicenseType, SailingLicense,
    SailingLicenseType,
};
use vereinsakte_core::office::{Office, Role};
use vereinsakte_core::requirement::{AnyOf, Qualification, Requirement};
use vereinsakte_core::reservation::{CrewSeat, Guest, Reservation};
use vereinsakte_core::sailing_permit::{AnnualRecord, Compensation, Reduction};
use vereinsakte_core::task::Task;
use vereinsakte_core::work_event::WorkEvent;
use vereinsakte_core::work_hours::{
    Confirmation, HourTransfer, WorkEntry, WorkExtent, WorkReference, WorkSubject,
};

use crate::db::StorageError;
use crate::values::*;

type Result<T> = std::result::Result<T, StorageError>;

/// Gruppiert Kindzeilen nach ihrem Elternschlüssel (Reihenfolge bleibt erhalten).
fn group<K: Eq + Hash, T>(
    rows: Vec<SqliteRow>,
    key: impl Fn(&SqliteRow) -> Result<K>,
    item: impl Fn(&SqliteRow) -> Result<T>,
) -> Result<HashMap<K, Vec<T>>> {
    let mut groups: HashMap<K, Vec<T>> = HashMap::new();
    for row in &rows {
        groups.entry(key(row)?).or_default().push(item(row)?);
    }
    Ok(groups)
}

/// Lädt die vollständige Vereinsakte.
pub(crate) async fn load_record(conn: &mut SqliteConnection) -> Result<ClubRecord> {
    Ok(ClubRecord {
        members: load_members(conn, &Filter::all()).await?,
        boats: load_boats(conn).await?,
        offices: load_offices(conn, &Filter::all()).await?,
        boat_checkouts: load_checkouts(conn, &Filter::all()).await?,
        revocations: load_revocations(conn, &Filter::all()).await?,
        work_entries: load_work_entries(conn, &Filter::all()).await?,
        hour_transfers: load_hour_transfers(conn, &Filter::all()).await?,
        work_events: load_work_events(conn).await?,
        tasks: load_tasks(conn).await?,
        seasons: load_seasons(conn, &Filter::all()).await?,
        annual_records: load_annual_records(conn, &Filter::all()).await?,
        reservations: load_reservations(conn).await?,
    })
}

pub(crate) async fn load_members(conn: &mut SqliteConnection, f: &Filter) -> Result<Vec<Member>> {
    let children = |table: &str| {
        format!(
            "SELECT * FROM {table} WHERE member_id IN (SELECT id FROM members {}) ORDER BY member_id, position",
            f.cond
        )
    };
    let mut sailing = group(
        rows(conn, &children("member_sailing_licenses"), &f.binds).await?,
        |r| get::<i64>(r, "member_id"),
        |r| {
            Ok(SailingLicense {
                kind: SailingLicenseType::from(get::<String>(r, "kind")?),
                acquired_at: get_opt_ts(r, "acquired_at")?,
            })
        },
    )?;
    let mut radio = group(
        rows(conn, &children("member_radio_licenses"), &f.binds).await?,
        |r| get::<i64>(r, "member_id"),
        |r| {
            Ok(RadioLicense {
                kind: RadioLicenseType::from(get::<String>(r, "kind")?),
                acquired_at: get_opt_ts(r, "acquired_at")?,
            })
        },
    )?;
    let mut claims = group(
        rows(conn, &children("claims"), &f.binds).await?,
        |r| get::<i64>(r, "member_id"),
        read_claim,
    )?;
    let mut payments = group(
        rows(conn, &children("payments"), &f.binds).await?,
        |r| get::<i64>(r, "member_id"),
        |r| {
            Ok(Payment {
                id: get_id(r, "id", PaymentId::from_uuid)?,
                amount: get_money(r, "amount_cents")?,
                at: get_ts(r, "at")?,
                text: get(r, "text")?,
            })
        },
    )?;
    rows(
        conn,
        &format!("SELECT * FROM members {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        let key: i64 = get(r, "id")?;
        Ok(Member {
            id: get_member(r, "id")?,
            last_name: get(r, "last_name")?,
            first_name: get(r, "first_name")?,
            address: Address {
                street: get(r, "street")?,
                postal_code: get(r, "postal_code")?,
                city: get(r, "city")?,
            },
            phone: PhoneNumbers {
                landline: get(r, "phone_landline")?,
                business: get(r, "phone_business")?,
                mobile: get(r, "phone_mobile")?,
            },
            email: get(r, "email")?,
            login_email: get(r, "login_email")?,
            address_valid: get_bool(r, "address_valid")?,
            membership_type: get_choice(r, "membership_type")?,
            child_of: get_opt_member(r, "child_of")?,
            board_consent: get_bool(r, "board_consent")?,
            membership: Membership {
                joined_at: get_ts(r, "joined_at")?,
                provisional_until: get_opt_ts(r, "provisional_until")?,
                limited_until: get_opt_ts(r, "limited_until")?,
                left_at: get_opt_ts(r, "left_at")?,
                expelled_at: get_opt_ts(r, "expelled_at")?,
            },
            sailing_licenses: sailing.remove(&key).unwrap_or_default(),
            radio_licenses: radio.remove(&key).unwrap_or_default(),
            account: MemberAccount {
                claims: claims.remove(&key).unwrap_or_default(),
                payments: payments.remove(&key).unwrap_or_default(),
            },
        })
    })
    .collect()
}

fn read_claim(r: &SqliteRow) -> Result<Claim> {
    let kind: String = get(r, "kind")?;
    let year = || -> Result<i32> {
        get::<Option<i64>>(r, "year")?
            .and_then(|y| i32::try_from(y).ok())
            .ok_or_else(|| StorageError::Corrupt("claims.year fehlt".into()))
    };
    let kind = match kind.as_str() {
        "annual_fee" => ClaimKind::AnnualFee { year: year()? },
        "admission_fee" => ClaimKind::AdmissionFee,
        "tour_fee" => ClaimKind::TourFee {
            reservation: get_id(r, "reservation_id", ReservationId::from_uuid)?,
            guest: get(r, "guest")?,
        },
        "work_compensation" => ClaimKind::WorkCompensation { year: year()? },
        "other" => ClaimKind::Other,
        other => return Err(StorageError::Corrupt(format!("claims.kind: {other}"))),
    };
    let cancelled = match get_opt_ts(r, "cancelled_at")? {
        Some(at) => Some(Cancellation {
            at,
            reason: get(r, "cancel_reason")?,
        }),
        None => None,
    };
    Ok(Claim {
        id: get_id(r, "id", ClaimId::from_uuid)?,
        kind,
        amount: get_money(r, "amount_cents")?,
        text: get(r, "text")?,
        created_at: get_ts(r, "created_at")?,
        due_at: get_opt_ts(r, "due_at")?,
        cancelled,
    })
}

fn read_qualification(r: &SqliteRow) -> Result<Qualification> {
    let kind: String = get(r, "kind")?;
    let license = || get::<String>(r, "license");
    Ok(match kind.as_str() {
        "sailing_license" => Qualification::SailingLicense(SailingLicenseType::from(license()?)),
        "radio_license" => Qualification::RadioLicense(RadioLicenseType::from(license()?)),
        "boat_permit" => Qualification::BoatPermit(get_id(r, "boat_ref", BoatId::from_uuid)?),
        "boat_permit_for_type" => Qualification::BoatPermitForType(get_choice(r, "boat_type")?),
        other => {
            return Err(StorageError::Corrupt(format!(
                "requirement_qualifications.kind: {other}"
            )));
        }
    })
}

pub(crate) async fn load_boats(conn: &mut SqliteConnection) -> Result<Vec<Boat>> {
    let mut qualifications = group(
        rows(
            conn,
            "SELECT * FROM requirement_qualifications ORDER BY group_id, position",
            &[],
        )
        .await?,
        |r| get::<i64>(r, "group_id"),
        read_qualification,
    )?;
    // (Boot, Fahrtgebiet oder None für den Skipper) -> Gruppen
    let mut groups = group(
        rows(
            conn,
            "SELECT * FROM requirement_groups ORDER BY boat_id, area, position",
            &[],
        )
        .await?,
        |r| {
            Ok((
                get::<String>(r, "boat_id")?,
                get::<Option<String>>(r, "area")?,
            ))
        },
        |r| get::<i64>(r, "id"),
    )?;
    let mut requirement = |boat: &str, area: Option<String>| -> Requirement {
        let ids = groups.remove(&(boat.to_string(), area)).unwrap_or_default();
        Requirement(
            ids.into_iter()
                .map(|id| AnyOf(qualifications.remove(&id).unwrap_or_default()))
                .collect(),
        )
    };
    let rule_rows = rows(
        conn,
        "SELECT * FROM crew_rules ORDER BY boat_id, position",
        &[],
    )
    .await?;
    let mut rules: HashMap<String, Vec<CrewRule>> = HashMap::new();
    for r in &rule_rows {
        let boat: String = get(r, "boat_id")?;
        let area: String = get(r, "area")?;
        let rule = CrewRule {
            area: parse_choice("crew_rules.area", &area)?,
            min_members: get_u8(r, "min_members")?,
            min_persons: get_u8(r, "min_persons")?,
            co_skipper_required: get_bool(r, "co_skipper_required")?,
            co_skipper_requirement: requirement(&boat, Some(area)),
        };
        rules.entry(boat).or_default().push(rule);
    }
    let mut releases = group(
        rows(
            conn,
            "SELECT * FROM boat_releases ORDER BY boat_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "boat_id"),
        |r| {
            Ok(Release {
                season: get::<i64>(r, "season")?
                    .try_into()
                    .map_err(|_| StorageError::Corrupt("season".into()))?,
                by: get_member(r, "by_member")?,
                at: get_ts(r, "at")?,
            })
        },
    )?;
    let mut blocks = group(
        rows(
            conn,
            "SELECT * FROM boat_blocks ORDER BY boat_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "boat_id"),
        |r| {
            Ok(Block {
                start: get_ts(r, "start_at")?,
                end: get_opt_ts(r, "end_at")?,
                reason: get(r, "reason")?,
                by: get_member(r, "by_member")?,
                lifted_by: get_opt_member(r, "lifted_by")?,
            })
        },
    )?;
    let boat_rows = rows(conn, "SELECT * FROM boats ORDER BY position", &[]).await?;
    let mut boats = Vec::with_capacity(boat_rows.len());
    for r in &boat_rows {
        let key: String = get(r, "id")?;
        boats.push(Boat {
            id: get_id(r, "id", BoatId::from_uuid)?,
            name: get(r, "name")?,
            boat_type: get_choice(r, "boat_type")?,
            class: get(r, "class")?,
            year_built: get_opt_num(r, "year_built")?,
            length_m: get(r, "length_m")?,
            beam_m: get(r, "beam_m")?,
            draft_m: get(r, "draft_m")?,
            sail_area_m2: get(r, "sail_area_m2")?,
            berths: get_opt_num(r, "berths")?,
            engine: get(r, "engine")?,
            mooring: get(r, "mooring")?,
            skipper_requirement: requirement(&key, None),
            crew_rules: rules.remove(&key).unwrap_or_default(),
            releases: releases.remove(&key).unwrap_or_default(),
            blocks: blocks.remove(&key).unwrap_or_default(),
        });
    }
    Ok(boats)
}

pub(crate) async fn load_offices(conn: &mut SqliteConnection, f: &Filter) -> Result<Vec<Office>> {
    rows(
        conn,
        &format!("SELECT * FROM offices {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        let boat = || get_id(r, "role_boat_id", BoatId::from_uuid);
        let role = match get::<String>(r, "role")?.as_str() {
            "boatwarden" => Role::Boatwarden(boat()?),
            "examiner" => Role::Examiner(boat()?),
            other => parse_choice("offices.role", other)?,
        };
        Ok(Office {
            id: get_id(r, "id", OfficeId::from_uuid)?,
            role,
            member: get_member(r, "member_id")?,
            start: get_ts(r, "start_at")?,
            end: get_opt_ts(r, "end_at")?,
        })
    })
    .collect()
}

pub(crate) async fn load_checkouts(
    conn: &mut SqliteConnection,
    f: &Filter,
) -> Result<Vec<BoatCheckout>> {
    rows(
        conn,
        &format!("SELECT * FROM boat_checkouts {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        Ok(BoatCheckout {
            id: get_id(r, "id", CheckoutId::from_uuid)?,
            member: get_member(r, "member_id")?,
            boat: get_id(r, "boat_id", BoatId::from_uuid)?,
            at: get_ts(r, "at")?,
            result: get_choice(r, "result")?,
            examiner: get_member(r, "examiner_id")?,
            remark: get(r, "remark")?,
        })
    })
    .collect()
}

pub(crate) async fn load_revocations(
    conn: &mut SqliteConnection,
    f: &Filter,
) -> Result<Vec<Revocation>> {
    rows(
        conn,
        &format!("SELECT * FROM revocations {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        Ok(Revocation {
            id: get_id(r, "id", RevocationId::from_uuid)?,
            member: get_member(r, "member_id")?,
            boat: get_id(r, "boat_id", BoatId::from_uuid)?,
            at: get_ts(r, "at")?,
            reason: get(r, "reason")?,
            by: get_member(r, "by_member")?,
        })
    })
    .collect()
}

pub(crate) async fn load_work_entries(
    conn: &mut SqliteConnection,
    f: &Filter,
) -> Result<Vec<WorkEntry>> {
    rows(
        conn,
        &format!("SELECT * FROM work_entries {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        let extent = match get_opt_ts(r, "date_at")? {
            Some(date) => WorkExtent::Amount {
                date,
                hours: get(r, "hours")?,
            },
            None => WorkExtent::Period {
                start: get_ts(r, "start_at")?,
                end: get_ts(r, "end_at")?,
            },
        };
        let subject = match get_opt_id(r, "subject_boat_id", BoatId::from_uuid)? {
            Some(boat) => WorkSubject::Boat(boat),
            None => WorkSubject::Club,
        };
        let reference = match (
            get_opt_id(r, "work_event_id", WorkEventId::from_uuid)?,
            get_opt_id(r, "task_id", TaskId::from_uuid)?,
        ) {
            (Some(event), _) => Some(WorkReference::WorkEvent(event)),
            (None, Some(task)) => Some(WorkReference::Task(task)),
            (None, None) => None,
        };
        let confirmation = match get_opt_member(r, "confirmed_by")? {
            Some(by) => Some(Confirmation {
                by,
                at: get_ts(r, "confirmed_at")?,
            }),
            None => None,
        };
        Ok(WorkEntry {
            id: get_id(r, "id", WorkEntryId::from_uuid)?,
            member: get_member(r, "member_id")?,
            extent,
            subject,
            description: get(r, "description")?,
            reference,
            confirmation,
        })
    })
    .collect()
}

pub(crate) async fn load_hour_transfers(
    conn: &mut SqliteConnection,
    f: &Filter,
) -> Result<Vec<HourTransfer>> {
    rows(
        conn,
        &format!("SELECT * FROM hour_transfers {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        Ok(HourTransfer {
            id: get_id(r, "id", HourTransferId::from_uuid)?,
            year: get_year(r, "year")?,
            from: get_member(r, "from_member")?,
            to: get_member(r, "to_member")?,
            hours: get(r, "hours")?,
            description: get(r, "description")?,
        })
    })
    .collect()
}

fn get_year(r: &SqliteRow, column: &str) -> Result<i32> {
    i32::try_from(get::<i64>(r, column)?)
        .map_err(|e| StorageError::Corrupt(format!("{column}: {e}")))
}

async fn load_work_events(conn: &mut SqliteConnection) -> Result<Vec<WorkEvent>> {
    let mut registrations = group(
        rows(
            conn,
            "SELECT * FROM work_event_registrations ORDER BY event_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "event_id"),
        |r| get_member(r, "member_id"),
    )?;
    rows(conn, "SELECT * FROM work_events ORDER BY position", &[])
        .await?
        .iter()
        .map(|r| {
            let key: String = get(r, "id")?;
            Ok(WorkEvent {
                id: get_id(r, "id", WorkEventId::from_uuid)?,
                title: get(r, "title")?,
                start: get_ts(r, "start_at")?,
                end: get_ts(r, "end_at")?,
                location: get(r, "location")?,
                description: get(r, "description")?,
                boat: get_opt_id(r, "boat_id", BoatId::from_uuid)?,
                max_participants: get_opt_num(r, "max_participants")?,
                registrations: registrations.remove(&key).unwrap_or_default(),
            })
        })
        .collect()
}

async fn load_tasks(conn: &mut SqliteConnection) -> Result<Vec<Task>> {
    let mut assignees = group(
        rows(
            conn,
            "SELECT * FROM task_assignees ORDER BY task_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "task_id"),
        |r| get_member(r, "member_id"),
    )?;
    rows(conn, "SELECT * FROM tasks ORDER BY position", &[])
        .await?
        .iter()
        .map(|r| {
            let key: String = get(r, "id")?;
            Ok(Task {
                id: get_id(r, "id", TaskId::from_uuid)?,
                title: get(r, "title")?,
                description: get(r, "description")?,
                boat: get_opt_id(r, "boat_id", BoatId::from_uuid)?,
                assignees: assignees.remove(&key).unwrap_or_default(),
                estimated_hours: get(r, "estimated_hours")?,
                due_at: get_opt_ts(r, "due_at")?,
                status: get_choice(r, "status")?,
                done_at: get_opt_ts(r, "done_at")?,
            })
        })
        .collect()
}

pub(crate) async fn load_seasons(conn: &mut SqliteConnection, f: &Filter) -> Result<Vec<Season>> {
    let mut type_rates = group(
        rows(conn, &format!("SELECT * FROM season_type_rates WHERE year IN (SELECT year FROM seasons {}) ORDER BY year, schedule, position", f.cond), &f.binds)
        .await?,
        |r| Ok((get::<i64>(r, "year")?, get::<String>(r, "schedule")?)),
        |r| {
            Ok(TypeRate {
                membership_type: get_choice(r, "membership_type")?,
                amount: get_money(r, "amount_cents")?,
            })
        },
    )?;
    let mut tour_rates = group(
        rows(conn, &format!("SELECT * FROM season_tour_rates WHERE year IN (SELECT year FROM seasons {}) ORDER BY year, position", f.cond), &f.binds)
        .await?,
        |r| get::<i64>(r, "year"),
        |r| {
            Ok(TourRate {
                tariff_group: get_choice(r, "tariff_group")?,
                day: get_money(r, "day_cents")?,
                week: get_money(r, "week_cents")?,
                first_week: get_opt_money(r, "first_week_cents")?,
            })
        },
    )?;
    rows(
        conn,
        &format!("SELECT * FROM seasons {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        let year: i64 = get(r, "year")?;
        let fees = if get_bool(r, "has_fee_schedule")? {
            Some(FeeSchedule {
                annual_fee: type_rates
                    .remove(&(year, "annual".into()))
                    .unwrap_or_default(),
                admission_fee: type_rates
                    .remove(&(year, "admission".into()))
                    .unwrap_or_default(),
                tour: tour_rates.remove(&year).unwrap_or_default(),
            })
        } else {
            None
        };
        Ok(Season {
            year: get_year(r, "year")?,
            hours_quota: get(r, "hours_quota")?,
            compensation_rate: get_opt_money(r, "compensation_rate_cents")?,
            fees,
        })
    })
    .collect()
}

pub(crate) async fn load_annual_records(
    conn: &mut SqliteConnection,
    f: &Filter,
) -> Result<Vec<AnnualRecord>> {
    rows(
        conn,
        &format!("SELECT * FROM annual_records {} ORDER BY position", f.cond),
        &f.binds,
    )
    .await?
    .iter()
    .map(|r| {
        let reduced_quota = match get::<Option<f64>>(r, "reduced_hours")? {
            Some(hours) => Some(Reduction {
                hours,
                reason: get(r, "reduction_reason")?,
            }),
            None => None,
        };
        let compensation = match get::<Option<f64>>(r, "compensation_hours")? {
            Some(hours) => Some(Compensation {
                hours,
                amount: get_opt_money(r, "compensation_cents")?,
                approved_at: get_ts(r, "compensation_approved_at")?,
                reason: get(r, "compensation_reason")?,
            }),
            None => None,
        };
        Ok(AnnualRecord {
            member: get_member(r, "member_id")?,
            year: get_year(r, "year")?,
            reduced_quota,
            compensation,
        })
    })
    .collect()
}

pub(crate) async fn load_reservations(conn: &mut SqliteConnection) -> Result<Vec<Reservation>> {
    let mut crew = group(
        rows(
            conn,
            "SELECT * FROM reservation_crew ORDER BY reservation_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "reservation_id"),
        |r| {
            Ok(CrewSeat {
                member: get_member(r, "member_id")?,
                from: get_opt_ts(r, "from_at")?,
                until: get_opt_ts(r, "until_at")?,
            })
        },
    )?;
    let mut guests = group(
        rows(
            conn,
            "SELECT * FROM reservation_guests ORDER BY reservation_id, position",
            &[],
        )
        .await?,
        |r| get::<String>(r, "reservation_id"),
        |r| {
            Ok(Guest {
                name: get(r, "name")?,
                phone: get(r, "phone")?,
                invited_by: get_opt_member(r, "invited_by")?,
                from: get_opt_ts(r, "from_at")?,
                until: get_opt_ts(r, "until_at")?,
            })
        },
    )?;
    rows(conn, "SELECT * FROM reservations ORDER BY position", &[])
        .await?
        .iter()
        .map(|r| {
            let key: String = get(r, "id")?;
            Ok(Reservation {
                id: get_id(r, "id", ReservationId::from_uuid)?,
                boat: get_id(r, "boat_id", BoatId::from_uuid)?,
                start: get_ts(r, "start_at")?,
                end: get_ts(r, "end_at")?,
                area: get_choice(r, "area")?,
                tour: get_bool(r, "tour")?,
                skipper: get_member(r, "skipper_id")?,
                co_skipper: get_opt_member(r, "co_skipper_id")?,
                crew: crew.remove(&key).unwrap_or_default(),
                guests: guests.remove(&key).unwrap_or_default(),
                remark: get(r, "remark")?,
            })
        })
        .collect()
}

//! Teildaten für Regeln, die mehrere Objekte brauchen (Reservierungsprüfung,
//! Beitrags- und Tourgebührenberechnung): Der Speicher liefert einen
//! `ClubRecord`, der nur die für den Fall nötigen Objekte enthält. Die
//! Fachregeln des Core laufen unverändert darauf.

use async_trait::async_trait;
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::ids::{BoatId, MemberId};
use vereinsakte_core::reservation::Reservation;
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;

/// Welche Mitglieder (mit Konto, Jahresnachweisen, Stunden, Abnahmen und
/// Widerrufen) in den Teildaten enthalten sein sollen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MemberScope {
    #[default]
    None,
    Some(Vec<MemberId>),
    All,
}

/// Reservierungen dieser Boote, die den Zeitraum berühren.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationWindow {
    pub boats: Vec<BoatId>,
    pub from: Timestamp,
    pub to: Timestamp,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub members: MemberScope,
    /// Boote, die enthalten sein sollen (`None`: keines).
    pub boats: Option<Vec<BoatId>>,
    pub reservations: Option<ReservationWindow>,
    /// Alle Saisons (Gebührensätze, Soll-Stunden).
    pub seasons: bool,
}

impl Scope {
    /// Alles, was die Prüfung einer Reservierung braucht: Boot, Beteiligte,
    /// Saisons und die Reservierungen des Bootes im Zeitraum.
    pub fn for_reservation(r: &Reservation) -> Self {
        Self {
            members: MemberScope::Some(r.members_on_board()),
            boats: Some(vec![r.boat]),
            reservations: Some(ReservationWindow {
                boats: vec![r.boat],
                from: r.start,
                to: r.end,
            }),
            seasons: true,
        }
    }

    /// Alle Mitglieder und Saisons (Jahresbeiträge anlegen).
    pub fn all_members() -> Self {
        Self {
            members: MemberScope::All,
            seasons: true,
            ..Self::default()
        }
    }
}

#[async_trait]
pub trait ScopeRepository: Send + Sync {
    async fn scope(&self, scope: &Scope) -> Result<ClubRecord, AppError>;
}

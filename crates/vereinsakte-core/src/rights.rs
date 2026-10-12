//! Rechte: Wer darf was? Einzige Quelle der Berechtigungsentscheidungen für
//! Fachregeln und Weboberfläche. Die Entscheidung stützt sich nur auf die
//! Ämter zum Zeitpunkt und die in der [`Capability`] mitgegebenen Daten.

use super::ids::{BoatId, MemberId};
use super::office::{NotAuthorized, Offices, Role};
use super::time::Timestamp;
use super::work_hours::WorkSubject;

/// Eine Handlung, für die geprüft wird, ob der Handelnde sie vornehmen darf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    // --- Boote (Spec `aemter`) ---
    BlockBoat(BoatId),
    LiftBlock(BoatId),
    ReleaseBoat(BoatId),
    RecordCheckout(BoatId),
    Revoke(BoatId),
    /// Bestätigung eines fremden Arbeitseinsatzes.
    ConfirmWork {
        owner: MemberId,
        subject: WorkSubject,
    },

    // --- Mitglieder ---
    /// Kontaktdaten, Segelscheine und Funklizenzen des Mitglieds ändern.
    EditProfile(MemberId),
    /// Kontakt, Segelerlaubnis, Stunden des Mitglieds lesen.
    ReadMember(MemberId),
    /// Mitglieder aufnehmen, Stammdaten, Mitgliedsart und Verlauf ändern.
    ManageMembers,

    // --- Ämter ---
    Appoint(Role),
    /// Ein Amt des Mitglieds beenden.
    Resign(MemberId),
    /// Archiv: Nicht mehr Aktuelles (beendete Ämter usw.).
    ReadArchive,

    // --- Boote (Web) ---
    CreateBoat,
    EditBoat(BoatId),

    // --- Arbeit ---
    /// Einen Arbeitseinsatz ändern oder löschen.
    EditWorkEntry {
        owner: MemberId,
        confirmed: bool,
    },
    ManageWorkEvents,
    ManageTasks,
    /// Status einer Aufgabe ändern.
    UpdateTask {
        assigned: bool,
    },
    /// Stundenüberträge und Abgeltungen.
    GrantHours,
    ReadAllHours,

    // --- Reservierungen ---
    Reserve {
        boat: BoatId,
        skipper: MemberId,
    },
    ChangeReservation {
        boat: BoatId,
        skipper: MemberId,
        started: bool,
    },

    // --- Finanzen ---
    ReadAccount(MemberId),
    ReadAllAccounts,
    PostFinance,

    // --- Protokoll ---
    ReadAudit,
}

/// Prüft Berechtigungen eines Handelnden zu einem Zeitpunkt.
#[derive(Clone, Copy, Debug)]
pub struct Authorizer<'a> {
    offices: Offices<'a>,
    actor: MemberId,
    at: Timestamp,
}

impl<'a> Authorizer<'a> {
    pub fn new(offices: Offices<'a>, actor: MemberId, at: Timestamp) -> Self {
        Self { offices, actor, at }
    }

    pub fn actor(&self) -> MemberId {
        self.actor
    }

    fn board(&self) -> bool {
        self.offices.is_board_member(self.actor, self.at)
    }

    fn auditor(&self) -> bool {
        self.offices
            .holds(self.actor, self.at, |r| r == Role::Auditor)
    }

    fn any_office(&self) -> bool {
        self.offices.holds(self.actor, self.at, |_| true)
    }

    fn boat_keeper(&self, boat: BoatId) -> bool {
        self.offices
            .holds(self.actor, self.at, |r| r == Role::Boatwarden(boat))
    }

    /// Vorstand oder Bootswart des Bootes.
    fn boat_manager(&self, boat: BoatId) -> bool {
        self.board() || self.boat_keeper(boat)
    }

    fn me(&self, member: MemberId) -> bool {
        self.actor == member
    }

    /// Darf der Handelnde die Handlung vornehmen?
    pub fn allows(&self, capability: &Capability) -> Result<(), NotAuthorized> {
        use Capability::*;
        let allowed = match *capability {
            BlockBoat(boat) | LiftBlock(boat) | ReleaseBoat(boat) => self.boat_manager(boat),
            RecordCheckout(boat) => {
                self.boat_manager(boat)
                    || self
                        .offices
                        .holds(self.actor, self.at, |r| r == Role::Examiner(boat))
            }
            Revoke(_) => self.board(),
            ConfirmWork { owner, subject } => {
                if self.me(owner) {
                    return Err(NotAuthorized::OwnWorkEntry);
                }
                match subject {
                    WorkSubject::Boat(boat) => self.boat_manager(boat),
                    WorkSubject::Club => self.any_office(),
                }
            }
            EditProfile(member) => self.me(member) || self.board(),
            ReadMember(member) | ReadAccount(member) => {
                self.me(member) || self.board() || self.auditor()
            }
            ManageMembers | Appoint(_) | CreateBoat | ManageWorkEvents | GrantHours
            | PostFinance => self.board(),
            Resign(member) => self.me(member) || self.board(),
            EditBoat(boat) => self.boat_manager(boat),
            EditWorkEntry { owner, confirmed } => {
                if confirmed {
                    self.board()
                } else {
                    self.me(owner) || self.board()
                }
            }
            ManageTasks => self.any_office(),
            UpdateTask { assigned } => assigned || self.any_office(),
            ReadAllHours | ReadAllAccounts | ReadAudit | ReadArchive => {
                self.board() || self.auditor()
            }
            Reserve { boat, skipper } => self.me(skipper) || self.boat_manager(boat),
            ChangeReservation {
                boat,
                skipper,
                started,
            } => {
                if started {
                    self.board()
                } else {
                    self.me(skipper) || self.boat_manager(boat)
                }
            }
        };
        if allowed {
            Ok(())
        } else {
            Err(NotAuthorized::MissingOffice)
        }
    }

    pub fn can(&self, capability: &Capability) -> bool {
        self.allows(capability).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example::{self, *};

    fn allowed(actor: MemberId, capability: Capability) -> bool {
        let club = example::club();
        let now = at(2026, 10, 9, 12, 0);
        Authorizer::new(club.office_queries(), actor, now).can(&capability)
    }

    /// (Handelnder, erlaubt?) je Handlung – die Rechtematrix der Spec `rechte`
    /// anhand der Beispielakte: Eva 1. Vorsitzende, Anna 2. Vorsitzende,
    /// Jonas Kassenwart, Karin Schriftwart, Frank Oberbootswart, Lukas und
    /// Clara Revisoren (zugleich Bootswarte der Trigon), Ben Bootswart der
    /// Harald Blauzahn, David Bootswart und Abnehmer der Filou, Greta ohne Amt.
    #[test]
    fn matrix_of_examples() {
        use Capability::*;
        let everyone = [
            ANNA, BEN, CLARA, DAVID, EVA, FRANK, GRETA, HANNES, JONAS, KARIN, LUKAS,
        ];
        let board = [EVA, ANNA, JONAS, KARIN, FRANK];
        let who = |cap: Capability| -> Vec<MemberId> {
            everyone
                .iter()
                .copied()
                .filter(|m| allowed(*m, cap))
                .collect()
        };
        let sorted = |mut v: Vec<MemberId>| {
            v.sort();
            v
        };

        // Vorstand und sonst niemand.
        for cap in [
            ManageMembers,
            Appoint(Role::Chair),
            Appoint(Role::Boatwarden(FILOU)),
            CreateBoat,
            ManageWorkEvents,
            GrantHours,
            PostFinance,
            Revoke(FILOU),
        ] {
            assert_eq!(sorted(who(cap)), sorted(board.to_vec()), "{cap:?}");
        }

        // Bootswart des Bootes, Oberbootswart und Vorstand sperren.
        let on_filou = who(BlockBoat(FILOU));
        assert!(on_filou.contains(&DAVID) && on_filou.contains(&EVA) && on_filou.contains(&FRANK));
        assert!(
            !on_filou.contains(&BEN),
            "Ben ist Bootswart der Harald Blauzahn"
        );
        assert!(!on_filou.contains(&GRETA));
        assert!(allowed(BEN, BlockBoat(HARALD_BLAUZAHN)));
        assert!(allowed(DAVID, EditBoat(FILOU)) && !allowed(DAVID, EditBoat(TRIGON)));

        // Abnahmen: Abnehmer, Bootswart oder Vorstand.
        assert!(allowed(DAVID, RecordCheckout(FILOU)));
        assert!(
            allowed(FRANK, RecordCheckout(HARALD_BLAUZAHN)),
            "Frank ist Abnehmer"
        );
        assert!(!allowed(GRETA, RecordCheckout(FILOU)));

        // Eigene Daten: Mitglied selbst oder Vorstand.
        assert!(allowed(GRETA, EditProfile(GRETA)) && !allowed(GRETA, EditProfile(CLARA)));
        assert!(allowed(EVA, EditProfile(CLARA)));
        assert!(allowed(GRETA, ReadMember(GRETA)) && !allowed(GRETA, ReadMember(CLARA)));
        assert!(allowed(LUKAS, ReadMember(CLARA)), "Revisor liest");
        assert!(
            !allowed(LUKAS, EditProfile(CLARA)),
            "Revisor schreibt nicht"
        );
        assert!(allowed(GRETA, Resign(GRETA)) && !allowed(GRETA, Resign(BEN)));

        // Finanzen: lesen für Vorstand und Revisor, Konto auch für sich selbst.
        assert!(allowed(GRETA, ReadAccount(GRETA)) && !allowed(GRETA, ReadAccount(CLARA)));
        assert!(allowed(LUKAS, ReadAllAccounts) && allowed(JONAS, ReadAllAccounts));
        assert!(!allowed(GRETA, ReadAllAccounts) && !allowed(BEN, ReadAllAccounts));
        assert!(
            !allowed(LUKAS, PostFinance),
            "Revisor schreibt keine Buchungen"
        );
        assert!(allowed(LUKAS, ReadAudit) && !allowed(GRETA, ReadAudit));
        assert!(
            allowed(LUKAS, ReadArchive)
                && allowed(EVA, ReadArchive)
                && !allowed(GRETA, ReadArchive)
                && !allowed(BEN, ReadArchive),
            "Archiv: Vorstand und Revisoren"
        );

        // Arbeit.
        let own = EditWorkEntry {
            owner: GRETA,
            confirmed: false,
        };
        assert!(allowed(GRETA, own) && !allowed(CLARA, own) && allowed(EVA, own));
        let confirmed = EditWorkEntry {
            owner: GRETA,
            confirmed: true,
        };
        assert!(!allowed(GRETA, confirmed) && allowed(EVA, confirmed));
        assert!(allowed(BEN, ManageTasks) && !allowed(GRETA, ManageTasks));
        assert!(allowed(GRETA, UpdateTask { assigned: true }));
        assert!(!allowed(GRETA, UpdateTask { assigned: false }));
        assert!(
            !allowed(
                BEN,
                ConfirmWork {
                    owner: BEN,
                    subject: WorkSubject::Club
                }
            ),
            "eigene Einsätze nicht selbst bestätigen"
        );
        assert!(allowed(
            BEN,
            ConfirmWork {
                owner: CLARA,
                subject: WorkSubject::Boat(HARALD_BLAUZAHN)
            }
        ));
        assert!(!allowed(
            BEN,
            ConfirmWork {
                owner: CLARA,
                subject: WorkSubject::Boat(FILOU)
            }
        ));

        // Reservierungen.
        assert!(allowed(
            GRETA,
            Reserve {
                boat: FILOU,
                skipper: GRETA
            }
        ));
        assert!(!allowed(
            GRETA,
            Reserve {
                boat: FILOU,
                skipper: ANNA
            }
        ));
        assert!(
            allowed(
                DAVID,
                Reserve {
                    boat: FILOU,
                    skipper: ANNA
                }
            ),
            "Bootswart reserviert für andere"
        );
        let future = ChangeReservation {
            boat: FILOU,
            skipper: ANNA,
            started: false,
        };
        assert!(allowed(ANNA, future) && !allowed(GRETA, future));
        let started = ChangeReservation {
            boat: FILOU,
            skipper: ANNA,
            started: true,
        };
        assert!(!allowed(DAVID, started) && allowed(EVA, started));
    }

    #[test]
    fn office_change_takes_effect_immediately() {
        let mut club = example::club();
        let t0 = at(2026, 10, 9, 12, 0);
        let sperren = Capability::BlockBoat(HARALD_BLAUZAHN);
        assert!(Authorizer::new(club.office_queries(), BEN, t0).can(&sperren));
        club.resign(
            BEN,
            Role::Boatwarden(HARALD_BLAUZAHN),
            at(2026, 10, 9, 13, 0),
        )
        .unwrap();
        let t1 = at(2026, 10, 9, 14, 0);
        assert!(!Authorizer::new(club.office_queries(), BEN, t1).can(&sperren));
        assert!(
            Authorizer::new(club.office_queries(), BEN, t0).can(&sperren),
            "vorher noch berechtigt"
        );
    }

    #[test]
    fn multiple_offices_combine_rights() {
        // Clara: Revisor und Bootswart der Trigon.
        assert!(allowed(CLARA, Capability::ReadAllAccounts));
        assert!(allowed(CLARA, Capability::BlockBoat(TRIGON)));
        assert!(!allowed(CLARA, Capability::PostFinance));
    }

    #[test]
    fn member_without_office_has_no_privileges() {
        for cap in [
            Capability::ManageMembers,
            Capability::CreateBoat,
            Capability::ReadAudit,
            Capability::PostFinance,
            Capability::ManageTasks,
        ] {
            assert!(!allowed(HANNES, cap), "{cap:?}");
        }
    }
}

//! Das handelnde Mitglied mit seinen Ämtern zum aktuellen Zeitpunkt. Aus den
//! Ämtern ergeben sich alle Rechte (Spec `rechte`); die Rolle wird bei jeder
//! Anfrage neu bestimmt und nie in der Sitzung gespeichert.

use vereinsakte_core::ids::MemberId;
use vereinsakte_core::office::{Office, Offices};
use vereinsakte_core::rights::{Authorizer, Capability};
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;
use crate::service::{ClubService, CurrentUser, office_labels};

#[derive(Clone, Debug)]
pub struct Principal {
    pub member: MemberId,
    pub name: String,
    /// Aktuelle Ämter als Text, Boote mit Namen.
    pub offices: Vec<String>,
    pub now: Timestamp,
    active_offices: Vec<Office>,
}

impl Principal {
    pub fn authorizer(&self) -> Authorizer<'_> {
        Authorizer::new(Offices::new(&self.active_offices), self.member, self.now)
    }

    pub fn can(&self, capability: &Capability) -> bool {
        self.authorizer().can(capability)
    }

    /// Bricht mit [`AppError::Forbidden`] ab, wenn die Handlung nicht erlaubt ist.
    pub fn require(&self, capability: &Capability) -> Result<(), AppError> {
        if self.can(capability) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn is_board(&self) -> bool {
        Offices::new(&self.active_offices).is_board_member(self.member, self.now)
    }

    pub fn has_office(&self) -> bool {
        !self.offices.is_empty()
    }

    /// Darf lesend auf alle Konten zugreifen (Vorstand, Revisor)?
    pub fn can_read_all_accounts(&self) -> bool {
        self.can(&Capability::ReadAllAccounts)
    }
}

impl ClubService {
    /// Bestimmt das handelnde Mitglied samt Ämtern. Nicht (mehr) aktive
    /// Mitglieder haben keinen Zugriff ([`AppError::Forbidden`]).
    pub async fn principal(&self, user: &CurrentUser) -> Result<Principal, AppError> {
        let now = self.now();
        let member = self
            .store
            .member(user.member)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {}", user.member)))?
            .value;
        if !member.is_active_at(now) {
            return Err(AppError::Forbidden);
        }
        let active_offices = self.store.active_offices(now).await?;
        let boats = self.store.boats().await?;
        let offices = office_labels(&Offices::new(&active_offices), &boats, user.member, now);
        Ok(Principal {
            member: user.member,
            name: member.full_name(),
            offices,
            now,
            active_offices,
        })
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{ANNA, BEN, GRETA, INES};
    use vereinsakte_core::time::club_time;

    use super::*;
    use crate::service::tests::service_at;

    #[tokio::test]
    async fn offices_and_rights_follow_the_clock() {
        let (service, _) = service_at(club_time(2026, 10, 9, 12, 0));
        let ben = service
            .principal(&CurrentUser { member: BEN })
            .await
            .unwrap();
        assert_eq!(ben.offices, ["Bootswart (Harald Blauzahn)"]);
        assert!(!ben.is_board() && ben.has_office());
        let anna = service
            .principal(&CurrentUser { member: ANNA })
            .await
            .unwrap();
        assert!(anna.is_board());
        assert!(anna.can(&Capability::ManageMembers));
        let greta = service
            .principal(&CurrentUser { member: GRETA })
            .await
            .unwrap();
        assert!(!greta.has_office());
        assert_eq!(
            greta.require(&Capability::ManageMembers),
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn former_members_have_no_principal() {
        let (service, _) = service_at(club_time(2026, 12, 1, 12, 0));
        let result = service.principal(&CurrentUser { member: INES }).await;
        assert!(matches!(result, Err(AppError::Forbidden)));
    }
}

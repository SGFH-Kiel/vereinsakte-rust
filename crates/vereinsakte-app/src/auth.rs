//! Zuordnung externer Anmeldeidentitäten zu Mitgliedern.

use std::fmt;
use std::sync::Arc;

use vereinsakte_core::ids::MemberId;

use crate::error::AppError;
use crate::ports::{Clock, IdentityRepository, MemberRepository};

/// Bereits geprüfte Angaben aus dem ID-Token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginClaims {
    /// Aussteller, z. B. `https://login.microsoftonline.com/{tenant}/v2.0`.
    pub provider: String,
    /// Unveränderliche Kennung des Kontos (Entra: `oid`).
    pub subject: String,
    /// Adresse des Kontos (`email`, ersatzweise `preferred_username`).
    pub email: Option<String>,
}

/// Grund, warum eine Anmeldung abgelehnt wird.
#[derive(Debug)]
pub enum LoginError {
    NoAddress,
    UnknownAccount,
    Inactive,
    Failed(AppError),
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HINT: &str = "Bitte wende dich an den Vorstand.";
        match self {
            Self::NoAddress => write!(f, "Dein Konto liefert keine E-Mail-Adresse. {HINT}"),
            Self::UnknownAccount => {
                write!(f, "Dieses Konto ist keinem Mitglied zugeordnet. {HINT}")
            }
            Self::Inactive => write!(f, "Deine Mitgliedschaft ist nicht aktiv. {HINT}"),
            Self::Failed(error) => write!(f, "Die Anmeldung ist fehlgeschlagen: {error}"),
        }
    }
}

impl From<AppError> for LoginError {
    fn from(error: AppError) -> Self {
        Self::Failed(error)
    }
}

#[derive(Clone)]
pub struct AuthService {
    members: Arc<dyn MemberRepository>,
    identities: Arc<dyn IdentityRepository>,
    clock: Arc<dyn Clock>,
}

impl AuthService {
    pub fn new(
        members: Arc<dyn MemberRepository>,
        identities: Arc<dyn IdentityRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            members,
            identities,
            clock,
        }
    }

    /// Ordnet eine Anmeldung einem aktiven Mitglied zu: über eine bestehende
    /// Verknüpfung, sonst über das Anmeldekonto (nie über die Kontakt-E-Mail).
    pub async fn resolve_login(&self, claims: &LoginClaims) -> Result<MemberId, LoginError> {
        let now = self.clock.now();
        if let Some(member) = self
            .identities
            .find(&claims.provider, &claims.subject)
            .await?
        {
            let active = self
                .members
                .member(member)
                .await?
                .is_some_and(|m| m.value.is_active_at(now));
            return if active {
                Ok(member)
            } else {
                Err(LoginError::Inactive)
            };
        }
        let address = claims
            .email
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .ok_or(LoginError::NoAddress)?;
        let member = self
            .members
            .member_by_login_email(address)
            .await?
            .ok_or(LoginError::UnknownAccount)?;
        if !member.is_active_at(now) {
            return Err(LoginError::Inactive);
        }
        self.identities
            .link(&claims.provider, &claims.subject, member.id)
            .await?;
        Ok(member.id)
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{self, ANNA, HANNES, INES};
    use vereinsakte_core::time::{Timestamp, club_time};

    use super::*;
    use crate::memory::{MemoryIdentityRepository, MemoryStore};
    use crate::ports::FixedClock;

    const ENTRA: &str = "https://login.microsoftonline.com/tenant/v2.0";

    fn auth_at(now: Timestamp) -> (AuthService, Arc<MemoryIdentityRepository>) {
        let identities = Arc::new(MemoryIdentityRepository::default());
        let service = AuthService::new(
            Arc::new(MemoryStore::new(example::club())),
            identities.clone(),
            Arc::new(FixedClock(now)),
        );
        (service, identities)
    }

    fn claims(subject: &str, email: Option<&str>) -> LoginClaims {
        LoginClaims {
            provider: ENTRA.into(),
            subject: subject.into(),
            email: email.map(Into::into),
        }
    }

    #[tokio::test]
    async fn first_login_links_by_login_account() {
        let (auth, identities) = auth_at(club_time(2026, 10, 9, 12, 0));
        let member = auth
            .resolve_login(&claims("oid-anna", Some("Anna.Petersen@SGFH.example")))
            .await
            .unwrap();
        assert_eq!(member, ANNA);
        assert_eq!(
            identities.find(ENTRA, "oid-anna").await.unwrap(),
            Some(ANNA)
        );

        // Spätere Anmeldung mit geänderter Adresse: Wiedererkennung über oid.
        let again = auth
            .resolve_login(&claims("oid-anna", Some("a.petersen@sgfh.example")))
            .await
            .unwrap();
        assert_eq!(again, ANNA);
    }

    #[tokio::test]
    async fn contact_email_does_not_log_in() {
        let (auth, _) = auth_at(club_time(2026, 10, 9, 12, 0));
        let result = auth
            .resolve_login(&claims("oid-x", Some("anna.petersen@example.org")))
            .await;
        assert!(matches!(result, Err(LoginError::UnknownAccount)));
        assert!(result.unwrap_err().to_string().contains("Vorstand"));
    }

    #[tokio::test]
    async fn missing_or_unknown_address() {
        let (auth, _) = auth_at(club_time(2026, 10, 9, 12, 0));
        assert!(matches!(
            auth.resolve_login(&claims("oid-1", None)).await,
            Err(LoginError::NoAddress)
        ));
        assert!(matches!(
            auth.resolve_login(&claims("oid-2", Some("niemand@sgfh.example")))
                .await,
            Err(LoginError::UnknownAccount)
        ));
        // Hannes hat kein Anmeldekonto.
        let record = example::club();
        assert!(record.member(HANNES).unwrap().login_email.is_none());
    }

    #[tokio::test]
    async fn inactive_member_is_rejected() {
        let (auth, identities) = auth_at(club_time(2026, 11, 15, 12, 0));
        let result = auth
            .resolve_login(&claims("oid-ines", Some("ines.dupont@sgfh.example")))
            .await;
        assert!(matches!(result, Err(LoginError::Inactive)));
        assert_eq!(identities.find(ENTRA, "oid-ines").await.unwrap(), None);

        // Auch eine bestehende Verknüpfung hilft nicht mehr.
        identities.link(ENTRA, "oid-ines", INES).await.unwrap();
        assert!(matches!(
            auth.resolve_login(&claims("oid-ines", None)).await,
            Err(LoginError::Inactive)
        ));
    }
}

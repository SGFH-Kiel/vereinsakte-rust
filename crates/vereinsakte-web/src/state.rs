//! Einstellungen und gemeinsamer Zustand der Web-Schicht.

use std::sync::Arc;
use std::time::Duration;

use vereinsakte_app::ClubService;
use vereinsakte_app::auth::AuthService;

use crate::oidc::LoginProvider;

/// Einstellungen der Web-Schicht (aus der Konfiguration des Programms).
#[derive(Clone, Debug)]
pub struct Settings {
    /// Entwicklungsmodus: Entwicklungs-Login und Benutzerwechsel, Cookies ohne `Secure`.
    pub development: bool,
    /// Öffentliche Adresse ohne abschließenden Schrägstrich.
    pub base_url: String,
    /// Sitzungen laufen nach dieser Zeit ohne Aktivität ab.
    pub session_idle: Duration,
    /// Tenant-ID des Entra-ID-Mandanten; nur Konten dieses Mandanten werden angenommen.
    pub tenant_id: Option<String>,
}

/// Zustand, den alle Handler teilen.
#[derive(Clone)]
pub struct AppState {
    pub club: ClubService,
    pub auth: AuthService,
    pub settings: Arc<Settings>,
    /// Anmeldung über Microsoft Entra ID; im Entwicklungsmodus `None`.
    pub login: Option<Arc<dyn LoginProvider>>,
}

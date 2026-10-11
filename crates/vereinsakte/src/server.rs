//! Zusammenbau von Datenbank, Diensten und Webserver.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use vereinsakte_app::ClubService;
use vereinsakte_app::auth::AuthService;
use vereinsakte_app::ports::SystemClock;
use vereinsakte_storage::{Database, SqliteClubStore, SqliteIdentityRepository};
use vereinsakte_web::oidc::EntraProvider;
use vereinsakte_web::{AppState, Settings};

use crate::config::{Config, Mode};

/// Baut den Router für eine bereits migrierte Datenbank.
pub async fn build_router(config: &Config, db: Database) -> Result<Router> {
    let development = config.mode == Mode::Development;
    let login: Option<Arc<dyn vereinsakte_web::oidc::LoginProvider>> = if development {
        None
    } else {
        let entra = config
            .entra
            .as_ref()
            .context("Konfiguration für Microsoft Entra ID fehlt")?;
        let provider = EntraProvider::discover(
            &entra.tenant_id,
            &entra.client_id,
            entra.client_secret.expose(),
            &config.base_url,
        )
        .await
        .map_err(anyhow::Error::msg)?;
        Some(Arc::new(provider))
    };

    let store = Arc::new(SqliteClubStore::new(db.clone()));
    let identities = Arc::new(SqliteIdentityRepository::new(db.clone()));
    let clock = Arc::new(SystemClock);
    let state = AppState {
        club: ClubService::new(store.clone(), clock.clone()),
        auth: AuthService::new(store, identities, clock),
        settings: Arc::new(Settings {
            development,
            base_url: config.base_url.trim_end_matches('/').to_owned(),
            session_idle: std::time::Duration::from_secs(config.session_idle_minutes * 60),
            tenant_id: config.entra.as_ref().map(|e| e.tenant_id.clone()),
        }),
        login,
    };
    Ok(vereinsakte_web::router(state, db.session_store()))
}

pub async fn serve(config: Config) -> Result<()> {
    let db = Database::connect(&config.database)
        .await
        .context("Datenbank lässt sich nicht öffnen")?;
    db.migrate().await.context("Migration fehlgeschlagen")?;
    let app = build_router(&config, db).await?;
    let address = std::net::SocketAddr::new(config.listen, config.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("{address} lässt sich nicht öffnen"))?;
    tracing::info!(%address, mode = ?config.mode, "Vereinsakte läuft");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

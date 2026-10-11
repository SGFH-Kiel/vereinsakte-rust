//! Konfiguration aus TOML-Datei und Umgebungsvariablen (`VEREINSAKTE_*`,
//! verschachtelte Werte mit `__`, z. B. `VEREINSAKTE_ENTRA__TENANT_ID`).

use std::fmt;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use figment::Figment;
use figment::providers::{Env, Format, Toml};
use serde::Deserialize;

/// Betriebsmodus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Entwicklungs-Login ohne Microsoft Entra ID, Cookies ohne `Secure`.
    Development,
    #[default]
    Production,
}

/// Geheimer Wert, der in `Debug`-Ausgaben maskiert wird.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"***\"")
    }
}

/// App-Registrierung in Microsoft Entra ID.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct EntraConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: SecretString,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub listen: IpAddr,
    pub port: u16,
    pub database: PathBuf,
    /// Öffentliche Adresse, z. B. `https://akte.segelgruppe-kiel.de`.
    pub base_url: String,
    pub mode: Mode,
    pub session_idle_minutes: u64,
    pub entra: Option<EntraConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: IpAddr::from([127, 0, 0, 1]),
            port: 8080,
            database: PathBuf::from("vereinsakte.db"),
            base_url: "http://localhost:8080".into(),
            mode: Mode::Production,
            session_idle_minutes: 8 * 60,
            entra: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Konfiguration ungültig: {0}")]
    Invalid(#[from] Box<figment::Error>),
    #[error("Im Produktionsmodus fehlt die Anmelde-Konfiguration: {0}")]
    MissingLogin(&'static str),
}

impl Config {
    /// Lädt die Konfiguration aus `file` (falls angegeben) und der Umgebung.
    pub fn load(file: Option<&Path>) -> Result<Self, ConfigError> {
        let mut figment = Figment::new();
        if let Some(file) = file {
            figment = figment.merge(Toml::file_exact(file));
        }
        let config: Config = figment
            .merge(Env::prefixed("VEREINSAKTE_").split("__"))
            .extract()
            .map_err(Box::new)?;
        config.check()?;
        Ok(config)
    }

    fn check(&self) -> Result<(), ConfigError> {
        if self.mode == Mode::Development {
            return Ok(());
        }
        let Some(entra) = &self.entra else {
            return Err(ConfigError::MissingLogin(
                "entra.tenant_id, entra.client_id, entra.client_secret",
            ));
        };
        if entra.tenant_id.trim().is_empty() {
            return Err(ConfigError::MissingLogin("entra.tenant_id"));
        }
        if entra.client_id.trim().is_empty() {
            return Err(ConfigError::MissingLogin("entra.client_id"));
        }
        if entra.client_secret.expose().is_empty() {
            return Err(ConfigError::MissingLogin("entra.client_secret"));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::result_large_err)] // Rückgabetyp von `Jail::expect_with` ist vorgegeben
mod tests {
    use figment::Jail;

    use super::*;

    #[test]
    fn environment_overrides_file() {
        Jail::expect_with(|jail| {
            jail.create_file("vereinsakte.toml", "mode = \"development\"\nport = 8080\n")?;
            jail.set_env("VEREINSAKTE_PORT", "9000");
            let config = Config::load(Some(Path::new("vereinsakte.toml"))).expect("Konfiguration");
            assert_eq!(config.port, 9000);
            assert_eq!(config.mode, Mode::Development);
            assert_eq!(config.entra, None, "Entwicklungsmodus ohne [entra] lädt");
            Ok(())
        });
    }

    #[test]
    fn nested_entra_values_from_environment() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "vereinsakte.toml",
                "[entra]\ntenant_id = \"aus-datei\"\nclient_id = \"app\"\nclient_secret = \"geheim\"\n",
            )?;
            jail.set_env("VEREINSAKTE_ENTRA__TENANT_ID", "aus-umgebung");
            let config = Config::load(Some(Path::new("vereinsakte.toml"))).expect("Konfiguration");
            assert_eq!(config.entra.unwrap().tenant_id, "aus-umgebung");
            Ok(())
        });
    }

    #[test]
    fn production_requires_login_configuration() {
        Jail::expect_with(|jail| {
            jail.create_file("vereinsakte.toml", "mode = \"production\"\n")?;
            let error = Config::load(Some(Path::new("vereinsakte.toml"))).unwrap_err();
            assert!(error.to_string().contains("entra.tenant_id"), "{error}");

            jail.create_file(
                "vereinsakte.toml",
                "[entra]\ntenant_id = \"\"\nclient_id = \"app\"\nclient_secret = \"geheim\"\n",
            )?;
            let error = Config::load(Some(Path::new("vereinsakte.toml"))).unwrap_err();
            assert!(error.to_string().ends_with("entra.tenant_id"), "{error}");
            Ok(())
        });
    }

    #[test]
    fn secret_is_masked_in_debug_output() {
        let config = Config {
            entra: Some(EntraConfig {
                tenant_id: "tenant".into(),
                client_id: "app".into(),
                client_secret: SecretString("sehr-geheim".into()),
            }),
            ..Config::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("sehr-geheim"), "{debug}");
        assert!(debug.contains("***"));
    }
}

//! Kommandozeile: `serve`, `migrate`, `import` und `export`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_storage::{Database, SqliteClubStore};

use crate::config::Config;
use crate::server;

#[derive(Debug, Parser)]
#[command(name = "vereinsakte", version, about = "Vereinsakte der Segelgruppe")]
pub struct Cli {
    /// Konfigurationsdatei (TOML); Werte lassen sich per `VEREINSAKTE_*` überschreiben.
    #[arg(long, short, env = "VEREINSAKTE_CONFIG", global = true)]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Webserver starten (legt fehlende Tabellen an).
    Serve,
    /// Datenbankschema anlegen bzw. auf den aktuellen Stand bringen.
    Migrate,
    /// Vereinsakte (JSON) in die Datenbank importieren.
    Import {
        file: PathBuf,
        /// Vorhandenen Bestand ersetzen.
        #[arg(long)]
        replace: bool,
    },
    /// Vereinsakte als JSON-Datei sichern.
    Export { file: PathBuf },
}

/// Initialisiert die Protokollierung (`RUST_LOG`, Standard: `info`).
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Einstiegspunkt; gibt den Exit-Code zurück.
pub fn run() -> std::process::ExitCode {
    let cli = Cli::parse();
    init_logging();
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Fehler: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match runtime.block_on(execute(cli)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Fehler: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

pub async fn execute(cli: Cli) -> Result<()> {
    let config = Config::load(cli.config.as_deref())?;
    match cli.command {
        Command::Serve => server::serve(config).await,
        Command::Migrate => migrate(&config.database).await,
        Command::Import { file, replace } => import(&config.database, &file, replace).await,
        Command::Export { file } => export(&config.database, &file).await,
    }
}

async fn open(database: &Path) -> Result<Database> {
    Database::connect(database)
        .await
        .with_context(|| format!("Datenbank {} lässt sich nicht öffnen", database.display()))
}

pub async fn migrate(database: &Path) -> Result<()> {
    let db = open(database).await?;
    db.migrate().await.context("Migration fehlgeschlagen")?;
    tracing::info!(database = %database.display(), "Schema ist aktuell");
    Ok(())
}

pub async fn import(database: &Path, file: &Path, replace: bool) -> Result<()> {
    let text = std::fs::read_to_string(file)
        .with_context(|| format!("{} lässt sich nicht lesen", file.display()))?;
    let record: ClubRecord = serde_json::from_str(&text)
        .with_context(|| format!("{} ist keine gültige Vereinsakte", file.display()))?;
    let db = open(database).await?;
    db.migrate().await.context("Migration fehlgeschlagen")?;
    SqliteClubStore::new(db)
        .import(&record, replace)
        .await
        .context("Import fehlgeschlagen")?;
    tracing::info!(members = record.members.len(), "Vereinsakte importiert");
    Ok(())
}

pub async fn export(database: &Path, file: &Path) -> Result<()> {
    let db = open(database).await?;
    let record = SqliteClubStore::new(db)
        .export()
        .await
        .context("Export fehlgeschlagen")?;
    let json = serde_json::to_string_pretty(&record)?;
    std::fs::write(file, json + "\n")
        .with_context(|| format!("{} lässt sich nicht schreiben", file.display()))?;
    tracing::info!(members = record.members.len(), "Vereinsakte exportiert");
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use vereinsakte_core::example;

    use super::*;

    #[test]
    fn unknown_command_is_rejected_with_help() {
        let error = Cli::try_parse_from(["vereinsakte", "frobnicate"]).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains("Usage"), "{error}");
    }

    #[tokio::test]
    async fn migrate_import_export_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("akte.db");
        let source = dir.path().join("quelle.json");
        let target = dir.path().join("sicherung.json");
        let record = example::club();
        std::fs::write(&source, serde_json::to_string_pretty(&record).unwrap()).unwrap();

        migrate(&db).await.unwrap();
        import(&db, &source, false).await.unwrap();
        export(&db, &target).await.unwrap();

        let exported: ClubRecord =
            serde_json::from_str(&std::fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(exported.members.len(), record.members.len());
        assert_eq!(
            serde_json::to_value(&exported).unwrap(),
            serde_json::to_value(&record).unwrap()
        );

        let error = import(&db, &source, false).await.unwrap_err();
        assert!(format!("{error:#}").contains("bereits Daten"), "{error:#}");
        import(&db, &source, true).await.unwrap();
    }
}

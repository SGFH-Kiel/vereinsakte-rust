//! Anwendungsschicht der Vereinsakte: Anwendungsfälle, Zugriffsprüfung und
//! Ports (Schnittstellen zu Speicher und Uhr). Kennt weder HTTP noch SQL.

pub mod audit;
pub mod auth;
pub mod changes;
pub mod directory;
pub mod error;
pub mod format;
pub mod memory;
pub mod my_data;
pub mod ports;
pub mod scope;
pub mod service;

pub use error::AppError;
pub use service::{ClubService, CurrentUser};

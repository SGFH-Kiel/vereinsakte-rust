//! Anwendungsschicht der Vereinsakte: Anwendungsfälle, Zugriffsprüfung und
//! Ports (Schnittstellen zu Speicher und Uhr). Kennt weder HTTP noch SQL.

pub mod audit;
pub mod auth;
pub mod changes;
pub mod commands;
pub mod directory;
pub mod error;
pub mod format;
pub mod members;
pub mod memory;
pub mod my_data;
pub mod ports;
pub mod principal;
pub mod scope;
pub mod service;

pub use error::{AppError, FieldError};
pub use principal::Principal;
pub use service::{ClubService, CurrentUser};

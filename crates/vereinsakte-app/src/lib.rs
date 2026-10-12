//! Anwendungsschicht der Vereinsakte: Anwendungsfälle, Zugriffsprüfung und
//! Ports (Schnittstellen zu Speicher und Uhr). Kennt weder HTTP noch SQL.

pub mod archive;
pub mod audit;
pub mod auth;
pub mod boat_images;
pub mod boats;
pub mod changes;
pub mod commands;
pub mod contact;
pub mod directory;
pub mod error;
pub mod finance;
pub mod format;
pub mod hours;
pub mod members;
pub mod memory;
pub mod my_data;
pub mod offices;
pub mod people;
pub mod ports;
pub mod principal;
pub mod reservations;
pub mod scope;
pub mod service;
pub mod task_extras;
pub mod tasks;
pub mod work;
pub mod work_plan;

pub use error::{AppError, FieldError};
pub use principal::Principal;
pub use service::{ClubService, CurrentUser};

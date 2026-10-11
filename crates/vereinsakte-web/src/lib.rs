//! Weboberfläche der Vereinsakte: axum-Router, askama-Templates, htmx,
//! Anmeldung über Microsoft Entra ID und Sitzungen.

mod boats;
pub mod error;
pub mod forms;
pub mod list;
mod login;
mod members;
mod offices;
pub mod oidc;
mod pages;
mod reservations;
mod router;
pub mod session;
pub mod state;
mod work;

pub use router::router;
pub use state::{AppState, Settings};

#[cfg(test)]
mod tests;

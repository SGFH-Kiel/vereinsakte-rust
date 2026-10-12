//! Weboberfläche der Vereinsakte: axum-Router, askama-Templates, htmx,
//! Anmeldung über Microsoft Entra ID und Sitzungen.

mod archive;
mod assets;
mod audit;
mod boats;
mod calendar;
pub mod error;
mod finance;
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

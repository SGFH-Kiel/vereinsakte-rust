//! Weboberfläche der Vereinsakte: axum-Router, askama-Templates, htmx,
//! Anmeldung über Microsoft Entra ID und Sitzungen.

pub mod error;
pub mod forms;
pub mod list;
mod login;
pub mod oidc;
mod pages;
mod router;
pub mod session;
pub mod state;

pub use router::router;
pub use state::{AppState, Settings};

#[cfg(test)]
mod tests;

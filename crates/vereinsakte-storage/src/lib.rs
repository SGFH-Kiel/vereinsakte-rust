//! SQLite-Speicherung der Vereinsakte in einem relationalen Schema.

mod boat_images;
mod commit;
pub mod db;
mod read;
pub mod repo;
mod task_extras;
mod values;
mod write;

pub use db::{Database, StorageError};
pub use repo::{SqliteClubStore, SqliteIdentityRepository};

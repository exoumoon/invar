#![allow(clippy::missing_errors_doc)]

use std::fmt;

use invar_pack::Pack;
use serde::{Deserialize, Serialize};

pub mod backup;
pub mod docker_compose;

pub const DEFAULT_MINECRAFT_PORT: u16 = 25565;

pub trait Server: fmt::Debug + Serialize + for<'de> Deserialize<'de> {
    type SetupError;
    type StartStopError;
    type StatusError;

    /// Prepare everything for the first start of the server.
    ///
    /// # Errors
    ///
    /// ...
    fn setup() -> Result<Self, Self::SetupError>;

    /// Start the hosted server, do nothing if it is already running.
    ///
    /// # Errors
    ///
    /// ...
    fn start(&self, pack: &Pack) -> Result<(), Self::StartStopError>;

    /// Stop the hosted server, do nothing if it is already stopped.
    ///
    /// # Errors
    ///
    /// ...
    fn stop(&self, pack: &Pack) -> Result<(), Self::StartStopError>;

    /// Report the status of the server.
    ///
    /// # Errors
    ///
    /// ...
    fn status(&self) -> Result<(), Self::StatusError> {
        todo!("Querying the server's status isn't yet implemented")
    }
}

/// The server's default `gamemode` for new players.
///
/// Variants are self-explanatory, I think...
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Gamemode {
    Survival,
    Creative,
    Hardcore,
    Spectator,
}

/// The server's difficulty level.
///
/// Variants are self-explanatory, I think...
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Difficulty {
    Peaceful,
    Easy,
    Medium,
    Hard,
}

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use invar_pack::Pack;

use crate::local::persist::PersistedEntity;

mod local;
mod modrinth;
pub use local::*;
pub use modrinth::*;

impl PersistedEntity for Pack {
    const FILE_PATH: &'static str = "pack.yml";
}

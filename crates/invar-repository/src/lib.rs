#![allow(clippy::missing_errors_doc)]

use invar_pack::Pack;

use crate::local::persist::PersistedEntity;

mod git;
mod local;
mod modrinth;
pub use git::*;
pub use local::*;
pub use modrinth::*;

impl PersistedEntity for Pack {
    const FILE_PATH: &'static str = "pack.yml";
}

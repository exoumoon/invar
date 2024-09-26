use crate::{instance::Instance, local_storage::PersistedEntity};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io;

/// The "modpack" entity.
///
/// A [`Pack`] represents a Minecraft [`Instance`] (with a [`Loader`](crate::instance::Loader)),
/// and all the mods that are in it, together with shaders, resourcepacks, configuration, etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    pub version: Version,
    pub authors: Vec<String>,

    /// The Minecraft [`Instance`] used in this modpack.
    pub instance: Instance,
}

impl PersistedEntity for Pack {
    const FILE_PATH: &'static str = "pack.yml";
}

impl Pack {
    /// Local path to the directory that stores the mods.
    pub const MOD_DIR: &'static str = "mods";

    /// Local path to the directory that stores the resourcepacks.
    pub const RESOURCEPACK_DIR: &'static str = "resourcepacks";

    /// Local path to the directory that stores the shaderpacks.
    pub const SHADERPACK_DIR: &'static str = "shaderpacks";

    /// Local path to the directory that stores the datapacks.
    pub const DATAPACK_DIR: &'static str = "datapacks";

    /// Local path to the directory that stores the configuration files.
    pub const CONFIG_DIR: &'static str = "config";

    /// Create the data subdirectories in the current directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if an I/O error occurs.
    pub fn setup_directories() -> io::Result<()> {
        for subdir in [
            Self::MOD_DIR,
            Self::RESOURCEPACK_DIR,
            Self::SHADERPACK_DIR,
            Self::DATAPACK_DIR,
            Self::CONFIG_DIR,
        ] {
            fs::create_dir_all(subdir)?;
            let _ = File::create(format!("{subdir}/.gitkeep"))?;
        }

        Ok(())
    }
}

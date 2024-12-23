use crate::index::{self, Index};
use crate::instance::Instance;
use crate::local_storage::{self, PersistedEntity};
use color_eyre::owo_colors::OwoColorize;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// The top-level "modpack" entity.
///
/// A [`Pack`] represents a Minecraft [`Instance`] (with a
/// [`Loader`](crate::instance::Loader)), and all the mods that are in it,
/// together with shaders, resourcepacks, configuration, etc.
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

    /// Local path to the directory that stores the shaders.
    pub const SHADERPACK_DIR: &'static str = "shaders";

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

    /// Export this [`Pack`]. See [`crate::index`] for details.
    ///
    /// # Errors
    ///
    /// This function may return a [`local_storage::Error`]. Look there for
    /// possible causes.
    pub fn export(&self) -> local_storage::Result<()> {
        let files: Vec<index::file::File> = crate::component::Component::load_all()?
            .into_iter()
            .map(Into::into)
            .collect();
        let index = Index::from_pack_and_files(self, &files);
        let json = serde_json::to_string_pretty(&index)?;
        let path = format!("{}.mrpack", self.name);

        tracing::info!(message = "Writing index", target = ?path.yellow().bold());
        let file = File::create(&path).map_err(|source| local_storage::Error::Io {
            source,
            faulty_path: Some(PathBuf::from(path.clone())),
        })?;
        let mut mrpack = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        mrpack.start_file("modrinth.index.json", options)?;
        mrpack
            .write_all(json.as_bytes())
            .map_err(|source| local_storage::Error::Io {
                source,
                faulty_path: Some(PathBuf::from(path)),
            })?;
        mrpack.finish()?;

        Ok(())
    }
}

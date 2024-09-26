use crate::{local_storage, pack::Pack};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::debug;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Mod,
    Resourcepack,
    Shaderpack,
    Datapack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    category: Category,
    slug: String,
    // version: String,
}

impl Component {
    pub const LOCAL_STORAGE_SUFFIX: &'static str = ".invar.yaml";

    /// Saves this [`Component`] in its metadata directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if a [`local_storage::Error`] occurs.
    pub fn save_to_metadata_dir(&self) -> Result<(), local_storage::Error> {
        let subdir = match self.category {
            Category::Mod => Pack::MOD_DIR,
            Category::Resourcepack => Pack::RESOURCEPACK_DIR,
            Category::Shaderpack => Pack::SHADERPACK_DIR,
            Category::Datapack => Pack::DATAPACK_DIR,
        };

        let yaml = serde_yml::to_string(self)?;
        fs::write(format!("{subdir}/{}", self.slug), yaml)?;

        Ok(())
    }
}

/// Load all [`Component`]s found in the metadata directories.
///
/// Only files with names ending in [`Component::LOCAL_STORAGE_SUFFIX`] will be loaded.
///
/// # Errors
///
/// This function will propagate errors occurring while reading
/// files or deserialing [`Component`]s from their contents.
#[tracing::instrument]
pub fn load_components() -> Result<Vec<Component>, local_storage::Error> {
    let mut components = vec![];

    for subdir in [
        Pack::MOD_DIR,
        Pack::RESOURCEPACK_DIR,
        Pack::SHADERPACK_DIR,
        Pack::DATAPACK_DIR,
    ] {
        for file in WalkDir::new(subdir)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
        {
            let path = file.path();
            if path
                .to_str()
                .is_some_and(|p| p.ends_with(Component::LOCAL_STORAGE_SUFFIX))
            {
                debug!(?path, "Found metadata file");
                let yaml = fs::read_to_string(path)?;
                let component = serde_yml::from_str(&yaml)?;
                components.push(component);
            }
        }
    }

    Ok(components)
}

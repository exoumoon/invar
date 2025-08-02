#![allow(clippy::missing_errors_doc)]

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use invar_component::{Component, LocalComponentEntry, Source};
use semver::Version;
use serde::{Deserialize, Serialize};
use settings::Settings;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::index::overrides::COMMON_OVERRIDES_DIR;
use crate::instance::Instance;

pub mod index;
pub mod instance;
pub mod settings;

/// The top-level **"modpack" entity**.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Pack {
    pub name: String,
    pub version: Version,
    pub instance: Instance,
    pub settings: Settings,

    #[serde(default)]
    pub local_components: Vec<LocalComponentEntry>,
}

impl Pack {
    pub const INDEX_FILE_NAME: &'static str = "modrinth.index.json";

    pub fn export<I>(&self, components: I, modpack_file_path: &PathBuf) -> Result<(), ExportError>
    where
        I: IntoIterator<Item = Component> + Clone,
    {
        let files = components
            .clone()
            .into_iter()
            .filter_map(|component| match component.source {
                Source::Remote(ref source) => {
                    let file = index::File::from_remote()
                        .runtime_path(component.runtime_path().into())
                        .env(component.environment)
                        .hashes(source.hashes.clone())
                        .remote_component(source.clone())
                        .build();
                    Some(file)
                }
                // Local components are handled as via overrides.
                Source::Local(_) => None,
            })
            .collect::<Vec<_>>();

        let index = index::Index::from_pack_and_files(self, files.as_slice());
        let json = serde_json::to_string(&index)?;

        let file = File::create(modpack_file_path)?;
        let options = SimpleFileOptions::default();
        let mut mrpack = ZipWriter::new(file);

        mrpack.start_file(Self::INDEX_FILE_NAME, options)?;
        mrpack.write_all(json.as_bytes())?;

        for component in components {
            if let Source::Local(local_component) = &component.source {
                let local_file_contents = std::fs::read(&local_component.path)?;
                let runtime_path = PathBuf::from(component.runtime_path());
                let runtime_path = runtime_path.to_string_lossy();
                mrpack.start_file(format!("{COMMON_OVERRIDES_DIR}/{runtime_path}"), options)?;
                mrpack.write_all(&local_file_contents)?;
            }
        }

        mrpack.finish()?;

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExportError {
    #[error("Failed to serialize the index to JSON")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to construct the .mrpack (zip archive)")]
    Zip(#[from] zip::result::ZipError),
    #[error("Failed to create the .mrpack file")]
    Io(#[from] std::io::Error),
}

#![feature(never_type)]

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use invar_component::{Component, Source};
use semver::Version;
use serde::{Deserialize, Serialize};
use settings::Settings;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::instance::Instance;

pub mod index;
pub mod instance;
pub mod settings;

/// The top-level **"modpack" entity**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    pub version: Version,
    pub instance: Instance,
    pub settings: Settings,
}

impl Pack {
    #[must_use]
    pub fn modpack_filename(&self) -> PathBuf {
        format!("{}.mrpack", self.name).into()
    }

    pub fn export(&self, components: &[Component]) -> Result<(), !> {
        let files = components
            .iter()
            .filter_map(|component| match &component.source {
                Source::Remote(source) => {
                    let file = index::File::from_remote()
                        .remote_component(source.clone())
                        .runtime_path(component.runtime_path().into())
                        .hashes(source.hashes.clone())
                        .env(component.environment.clone())
                        .build();
                    Some(file)
                }
                // skip this shit for now
                Source::Local(_) => None,
            })
            .collect::<Vec<_>>();

        let index = index::Index::from_pack_and_files(self, files.as_slice());
        let json = serde_json::to_string(&index).unwrap();

        let file = File::create(self.modpack_filename()).unwrap();
        let mut mrpack = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        mrpack.start_file("modrinth.index.json", options).unwrap();
        mrpack.write_all(json.as_bytes()).unwrap();
        mrpack.finish().unwrap();

        Ok(())
    }
}

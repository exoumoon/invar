#![feature(never_type)]

use semver::Version;
use serde::{Deserialize, Serialize};
use settings::Settings;

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
    #[expect(clippy::missing_errors_doc)]
    pub fn export(&self) -> Result<(), !> {
        // let file = File::create(&path).map_err(|source| local_storage::Error::Io {
        //     source,
        //     faulty_path: Some(PathBuf::from(path.clone())),
        // })?;
        // let mut mrpack = ZipWriter::new(file);
        // let options =
        //     SimpleFileOptions::default().
        // compression_method(zip::CompressionMethod::Deflated);
        // mrpack.start_file("modrinth.index.json", options)?;
        // mrpack
        //     .write_all(json.as_bytes())
        //     .map_err(|source| local_storage::Error::Io {
        //         source,
        //         faulty_path: Some(PathBuf::from(path)),
        //     })?;
        // mrpack.finish()?;

        todo!("Pack exporting is not yet wired up")
    }
}

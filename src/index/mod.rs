/// An entity representing a single project in the `files` array.
pub mod file;

use crate::{instance::Loader, pack::Pack};
use file::File;
use semver::Version;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Index<'pack, 'files> {
    pub dependencies: HashMap<Loader, Version>,
    pub files: &'files [File],
    pub format_version: u8,
    pub game: &'static str,
    pub name: &'pack str,
    pub version_id: &'pack Version,
}

impl<'pack, 'files> Index<'pack, 'files> {
    const GAME_LITERAL: &'static str = "minecraft";
    const FORMAT_VERSION: u8 = 1;
}

impl<'pack, 'files> Index<'pack, 'files> {
    #[must_use]
    pub fn from_pack_and_files(pack: &'pack Pack, files: &'files [File]) -> Self {
        Self {
            game: Self::GAME_LITERAL,
            format_version: Self::FORMAT_VERSION,
            version_id: &pack.version,
            name: &pack.name,
            dependencies: pack.instance.index_dependencies(),
            files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{file::File, Index};
    use crate::instance::{Instance, Loader};
    use crate::pack::Pack;
    use semver::Version;

    // As long as this compiles and runs, its fine.
    #[test]
    fn raw_creation() {
        let pack = Pack {
            name: String::from("sample_pack"),
            version: Version::new(0, 1, 0),
            authors: vec![String::from("mxxntype")],
            instance: Instance {
                minecraft_version: Version::new(1, 20, 1),
                loader: Loader::Neoforge,
                loader_version: Version::new(47, 3, 7),
                allowed_foreign_loaders: vec![Loader::Forge],
            },
        };

        let files: Vec<File> = vec![];
        let index = Index::from_pack_and_files(&pack, &files);
        let json = serde_json::to_string_pretty(&index).unwrap();
        eprintln!("{json}");
    }
}

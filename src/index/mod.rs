/// An entity representing a single project in the `files` array.
pub mod file;

use crate::instance::Loader;
use crate::pack::Pack;
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

impl Index<'_, '_> {
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

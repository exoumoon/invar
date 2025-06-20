use std::collections::HashMap;
use std::path::PathBuf;

use bon::bon;
use invar_component::{Env, Hashes, RemoteComponent};
use overrides::Overrides;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::instance::Loader;
use crate::Pack;

/// Interface for with the `overrides` folder inside of an **`.mrpack`**.
pub mod overrides;

/// [Modrinth's `.mrpack`](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack) format structure.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Index<
    'pack,  // This lifetime represents borrows from a `Pack`.
    'files, // This lifetime represents borrows from a `&[File]`.
> {
    pub dependencies: HashMap<Loader, String>,
    pub files: &'files [File],
    pub format_version: u8,
    pub game: &'static str,
    pub name: &'pack str,
    pub version_id: &'pack Version,

    #[serde(skip_serializing)]
    pub overrides: Overrides,
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
            overrides: Overrides::default(),
        }
    }
}

/// An entry in the `files` array of the [`Index`].
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub path: PathBuf,
    pub hashes: Hashes,
    pub env: Env,
    pub downloads: Vec<Url>,
    pub file_size: usize,
}

#[bon]
impl File {
    #[builder(finish_fn = build)]
    pub fn from_remote(
        runtime_path: PathBuf,
        hashes: Hashes,
        env: Env,
        remote_component: RemoteComponent,
    ) -> Self {
        Self {
            path: runtime_path,
            hashes,
            env,
            downloads: vec![remote_component.download_url],
            file_size: remote_component.file_size,
        }
    }
}

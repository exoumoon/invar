use crate::index::file::{Env, Hashes, Requirement};
use crate::instance::{Instance, Loader};
use crate::local_storage;
use crate::pack::Pack;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::debug;
use url::Url;

/// Possible types (categories) of [`Component`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Mod,
    Resourcepack,
    Shaderpack,
    Datapack,
}

/// Errors that may arise when adding a new [`Component`].
#[derive(thiserror::Error, Debug)]
pub enum AddError {
    #[error("API error: {0:?}")]
    Api(#[from] reqwest::Error),
    #[error("Could not find a compatible version of this component")]
    Incompatible,
    #[error("The latest compatible version of this component has no files associated")]
    NoFile,
}

/// A (runtime) modpack component.
///
/// A component is one of the elements that go into the `files` array of the `.mrpack` index.
/// These usually represent mods, resourcepacks, shaderpacks, datapacks, but can be anything,
/// if needed. New components are obtained from the **Modrinth API**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub slug: String,
    pub category: Category,
    pub environment: Env,
    pub version_id: String,
    pub file_name: String,
    pub file_size: usize,
    pub download_url: Url,
    pub hashes: Hashes,
}

impl Component {
    /// The suffix (secondary file extension) for local metadata files.
    pub const LOCAL_STORAGE_SUFFIX: &'static str = ".invar.yaml";

    /// Saves this [`Component`] in its metadata directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if a [`local_storage::Error`] occurs.
    pub fn save_to_metadata_dir(&self) -> Result<(), local_storage::Error> {
        let yaml = serde_yml::to_string(self)?;
        fs::write(self.local_storage_path(), yaml)?;

        Ok(())
    }

    /// Construct a path where this component should be stored.
    #[must_use]
    pub fn local_storage_path(&self) -> PathBuf {
        format!(
            "{subdir}/{slug}{suffix}",
            subdir = match self.category {
                Category::Mod => Pack::MOD_DIR,
                Category::Resourcepack => Pack::RESOURCEPACK_DIR,
                Category::Shaderpack => Pack::SHADERPACK_DIR,
                Category::Datapack => Pack::DATAPACK_DIR,
            },
            slug = self.slug,
            suffix = Self::LOCAL_STORAGE_SUFFIX
        )
        .into()
    }

    /// Fetch a [`Component`] from the **Modrinth API**.
    ///
    /// The process is:
    /// 1. Get the component's available versions from [`/project/{id|slug}/version`](https://docs.modrinth.com/#tag/versions/operation/getProjectVersions).
    /// 2. Filter the versions based on the `loaders` and `game_versions` fields.
    /// 3. Pick the latest from the compatible ones.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - It fails to query the Modrinth API;
    /// - None of the versions of the component are compatible with the provided [`Instance`];
    /// - There are no URLs to where the component's file can be downloaded (unlikely...)
    pub fn fetch_from_modrinth(slug: &str, instance: &Instance) -> Result<Self, AddError> {
        #[derive(Deserialize, Debug)]
        struct File {
            hashes: Hashes,
            url: Url,
            filename: String,
            size: usize,
        }

        #[derive(Deserialize, Debug, Clone, Copy)]
        struct Metadata {
            #[serde(rename = "project_type")]
            category: Category,
            client_side: Requirement,
            server_side: Requirement,
        }

        #[derive(Deserialize, Debug)]
        struct Version {
            id: String,
            game_versions: Vec<String>,
            loaders: Vec<Loader>,
            date_published: chrono::DateTime<chrono::Utc>,
            files: Vec<File>,
        }

        let metadata_url = format!("https://api.modrinth.com/v2/project/{slug}");
        let versions_url = format!("https://api.modrinth.com/v2/project/{slug}/version");
        let metadata: Metadata = reqwest::blocking::get(metadata_url)?.json()?;
        let mut versions: Vec<Version> = reqwest::blocking::get(versions_url)?.json()?;

        // Only leave versions that are both loader- and version-compatible with the instance.
        versions.retain(|v| {
            let version_compatible = v.game_versions.iter().any(|v| {
                semver::Version::from_str(v).is_ok_and(|v| v == instance.minecraft_version)
            });
            let loader_compatible = v
                .loaders
                .iter()
                .any(|l| *l == instance.loader || instance.allowed_foreign_loaders.contains(l));
            loader_compatible && version_compatible
        });
        versions.sort_unstable_by_key(|version| version.date_published);

        if let Some(latest_compatible_version) = versions.last() {
            let main_file = latest_compatible_version
                .files
                .first()
                .ok_or(AddError::NoFile)?;
            Ok(Self {
                slug: slug.to_owned(),
                category: metadata.category,
                version_id: latest_compatible_version.id.clone(),
                file_name: main_file.filename.clone(),
                file_size: main_file.size,
                download_url: main_file.url.clone(),
                hashes: main_file.hashes.clone(),
                environment: Env {
                    client: metadata.client_side,
                    server: metadata.server_side,
                },
            })
        } else {
            Err(AddError::Incompatible)
        }
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

    for file in local_storage::metadata_files(".")? {
        let path = file.path();
        debug!(?path, "Found metadata file");
        let yaml = fs::read_to_string(path)?;
        let component = serde_yml::from_str(&yaml)?;
        components.push(component);
    }

    Ok(components)
}

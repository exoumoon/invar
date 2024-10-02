use crate::index::file::{Env, Hashes, Requirement};
use crate::instance::{Instance, Loader};
use crate::local_storage;
use color_eyre::owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::{fs, path::PathBuf, str::FromStr};
use strum::Display;
use tracing::debug;
use url::Url;

/// Possible types (categories) of [`Component`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Mod,
    Resourcepack,
    Shaderpack,
    Datapack,
}

impl Category {
    #[must_use]
    pub fn to_runtime_path(&self) -> PathBuf {
        PathBuf::from(match self {
            Self::Mod => "mods",
            Self::Resourcepack => "resourcepacks",
            Self::Shaderpack => "shaderpacks",
            Self::Datapack => "datapacks",
        })
    }
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
    #[error("Failed to get required input from user")]
    User(#[from] inquire::error::InquireError),
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
        let mut path = self.category.to_runtime_path();
        path.push(format!("{}{}", self.slug, Self::LOCAL_STORAGE_SUFFIX));
        path
    }

    /// Construct a path where this component should be at runtime.
    #[must_use]
    pub fn runtime_path(&self) -> PathBuf {
        let mut path = self.category.to_runtime_path();
        path.push(&self.file_name);
        path
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
            name: String,
            game_versions: Vec<String>,
            loaders: Vec<Loader>,
            date_published: chrono::DateTime<chrono::Utc>,
            files: Vec<File>,
        }

        impl fmt::Display for Version {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "{name} [ID: {id}] - Supported loaders: {loaders:?}, released: {date}",
                    name = self.name.yellow().bold(),
                    id = self.id.bold(),
                    loaders = self.loaders.bright_red(),
                    date = self.date_published.format("%b %e, %Y").bright_blue().bold()
                )?;
                Ok(())
            }
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
        versions.reverse();

        let version = match versions.len() {
            0 => return Err(AddError::Incompatible),
            1 => versions.first().unwrap_or_else(|| unreachable!()),
            count => {
                let message = format!(
                    "{count} compatible versions of {} found, choose one:",
                    slug.magenta().bold()
                );
                let help = format!(
                    "NOTE: this component will be added as a '{}', so pick a version with the right loaders",
                    metadata.category
                );
                &inquire::Select::new(&message, versions)
                    .with_help_message(&help)
                    .prompt()?
            }
        };

        let file = version.files.first().ok_or(AddError::NoFile)?;
        Ok(Self {
            slug: slug.to_owned(),
            category: metadata.category,
            environment: Env {
                client: metadata.client_side,
                server: metadata.server_side,
            },
            version_id: version.id.clone(),
            file_name: file.filename.clone(),
            file_size: file.size,
            download_url: file.url.clone(),
            hashes: file.hashes.clone(),
        })
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

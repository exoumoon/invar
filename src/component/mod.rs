use crate::index::file::{Env, Hashes};
use crate::instance::{Instance, Loader};
use crate::local_storage;
use color_eyre::owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};
use strum::Display;
use tracing::debug;
use url::Url;

mod tag;
pub use tag::*;

/// [Modrinth](https://modrinth.com)-specific code.
pub mod modrinth;

/// A (runtime) modpack component.
///
/// A component is one of the elements that go into the `files` array of the
/// `.mrpack` index. These usually represent mods, resourcepacks, shaderpacks,
/// datapacks, but can be anything, if needed. New components are obtained from
/// the **Modrinth API**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub slug: String,
    pub category: Category,
    pub tags: tag::TagInformation,
    pub environment: Env,
    pub version_id: String,
    pub file_name: String,
    pub file_size: usize,
    pub download_url: Url,
    pub hashes: Hashes,
}

/// Possible types (categories) of [`Component`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Mod,
    Resourcepack,
    #[serde(alias = "shaderpack")]
    Shader,
    Datapack,
    Config,
}

impl Component {
    /// The suffix (secondary file extension) for local metadata files.
    pub const LOCAL_STORAGE_SUFFIX: &'static str = ".invar.yaml";

    /// Load all [`Component`]s found in the metadata directories.
    ///
    /// Only files with names ending in [`Component::LOCAL_STORAGE_SUFFIX`] will
    /// be loaded.
    ///
    /// # Errors
    ///
    /// This function will propagate errors occurring while reading
    /// files or deserialing [`Component`]s from their contents.
    #[tracing::instrument]
    pub fn load_all() -> Result<Vec<Self>, local_storage::Error> {
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

    /// Remove a [`Component`] by slug.
    ///
    /// # Errors
    ///
    /// This function will return an error if there are no components with this
    /// slug or an error occurs when deleting it.
    pub fn remove(slug: &str) -> Result<(), local_storage::Error> {
        let target_filename = format!("{slug}{}", Self::LOCAL_STORAGE_SUFFIX);
        let candidate = local_storage::metadata_files(".")?.find(|dir_entry| {
            dir_entry
                .file_name()
                .to_str()
                .is_some_and(|name| name == target_filename)
        });
        match candidate {
            Some(file) => fs::remove_file(file.path())?,
            None => return Err(io::Error::new(ErrorKind::NotFound, "Failed to find file").into()),
        }

        Ok(())
    }

    /// Saves this [`Component`] in its metadata directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if a [`local_storage::Error`] occurs.
    ///
    /// # Panics
    ///
    /// This function will panic if the [parent](std::path::Path::parent) of
    /// this [`Component`]'s [local storage path](Self::local_storage_path)
    /// ends up being [`None`], which shouldn't happen.
    pub fn save_to_metadata_dir(&self) -> Result<(), local_storage::Error> {
        let yaml = serde_yml::to_string(self)?;
        let path = self.local_storage_path();
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, yaml)?;

        Ok(())
    }

    /// Construct a path where this component should be stored.
    #[must_use]
    pub fn local_storage_path(&self) -> PathBuf {
        let mut path = PathBuf::from(self.category);
        if let Some(tag) = &self.tags.main {
            path.push(tag.to_string());
        }
        path.push(format!("{}{}", self.slug, Self::LOCAL_STORAGE_SUFFIX));
        path
    }

    /// Construct a path where this component should be at runtime.
    #[must_use]
    pub fn runtime_path(&self) -> PathBuf {
        let mut path = PathBuf::from(self.category);
        path.push(&self.file_name);
        path
    }

    /// Fetch a [`Component`] from the **Modrinth API**.
    ///
    /// The process is:
    /// 1. Get the component's available versions from [`/project/{id|slug}/version`](https://docs.modrinth.com/#tag/versions/operation/getProjectVersions).
    /// 2. Filter the versions based on the `loaders` and `game_versions`
    ///    fields.
    /// 3. Pick the latest from the compatible ones.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - It fails to query the Modrinth API;
    /// - None of the versions of the component are compatible with the provided
    ///   [`Instance`];
    /// - There are no URLs to where the component's file can be downloaded
    ///   (unlikely...)
    #[tracing::instrument]
    pub fn fetch_from_modrinth(slug: &str, instance: &Instance) -> Result<Self, AddError> {
        let metadata_url = format!("https://api.modrinth.com/v2/project/{slug}");
        let versions_url = format!("https://api.modrinth.com/v2/project/{slug}/version");
        let metadata: modrinth::Metadata = reqwest::blocking::get(metadata_url)?.json()?;
        let mut versions: Vec<modrinth::Version> = reqwest::blocking::get(versions_url)?.json()?;

        // Only leave versions that are both loader- and version-compatible with the
        // instance.
        versions.retain(|v| {
            // Resourcepacks and shaders may be loaded even if they are made for a different
            // version.
            let version_insensitive =
                [Category::Resourcepack, Category::Shader].contains(&metadata.category);
            let version_compatible = v.game_versions.iter().any(|v| {
                semver::Version::from_str(v).is_ok_and(|v| v == instance.minecraft_version)
            });
            let version_compatible = version_insensitive || version_compatible;
            let loader_compatible = v.loaders.iter().any(|l| {
                *l == instance.loader
                    || instance.allowed_foreign_loaders.contains(l)
                    || *l == Loader::Other
            });
            loader_compatible && version_compatible
        });

        for version in &mut versions {
            version.loaders.dedup();
        }
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
        let main_tag = self::tag::pick_main_tag()?;
        let other_tags = self::tag::pick_secondary_tags(main_tag.as_ref())?;
        let component = Self {
            slug: slug.to_owned(),
            category: metadata.category,
            tags: tag::TagInformation {
                main: main_tag,
                others: other_tags,
            },
            environment: Env {
                client: metadata.client_side,
                server: metadata.server_side,
            },
            version_id: version.id.clone(),
            file_name: file.filename.clone(),
            file_size: file.size,
            download_url: file.url.clone(),
            hashes: file.hashes.clone(),
        };

        Ok(component)
    }
}

/// This [`From`] implementation represents the [`Category`] to `folder
/// in minecraft's data directory` transformation.
impl From<Category> for PathBuf {
    fn from(category: Category) -> Self {
        Self::from(match category {
            Category::Mod => "mods",
            Category::Resourcepack => "resourcepacks",
            Category::Datapack => "datapacks",
            // WARN: We do keep it in `shaders/` in local storage, but Minecraft expects it in
            // `shaderpacks/`.
            Category::Shader => "shaderpacks",
            Category::Config => "config",
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

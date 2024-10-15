use crate::index::file::{Env, Hashes};
use crate::instance::{Instance, Loader};
use crate::local_storage;
use color_eyre::owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, str::FromStr};
use strum::{Display, EnumIter, IntoEnumIterator};
use tracing::debug;
use url::Url;

/// Possible types (categories) of [`Component`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Mod,
    Resourcepack,
    #[serde(alias = "shaderpack")]
    Shader,
    Datapack,
}

impl From<Category> for PathBuf {
    fn from(category: Category) -> Self {
        Self::from(match category {
            Category::Mod => "mods",
            Category::Resourcepack => "resourcepacks",
            Category::Datapack => "datapacks",
            // WARN: We do keep it in `shaders/` in local storage, but Minecraft expects it in `shaderpacks/`.
            Category::Shader => "shaderpacks",
        })
    }
}

/// Possible tags that can be associated with a [`Component`].
///
/// A [`Component`] would usually have a "main" tag and "other" tags.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Display, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    /// Stuff that adds weapons and/or combat mechanics, like **Better Combat**.
    Combat,
    /// Stuff that adds compatiblity between other components and/or [`Loader`]s.
    Compatibility,
    /// An uncategorized tag added by the user.
    #[strum(to_string = "{0}")]
    Custom(String),
    /// Stuff that adds new or modifies existing dimensions.
    Dimensions,
    /// Stuff that adds new food, crops and animals, like **Farmer's Delight**.
    Farming,
    /// Stuff that adds new weapons, tools and armor.
    Gear,
    /// Libraries for other components, like **Cloth Config API** or **Zeta**.
    Library,
    /// Stuff that adds new hostile mobs to the game, like **Born in Chaos**.
    Mobs,
    /// Overworld generation stuff, like **Tectonic** and **Geophilic**.
    Overworld,
    /// Stuff that improves the game's performance, like **Sodium**.
    Performance,
    /// Stuff that tweaks the game's progression, like **Improvable Skills**.
    Progression,
    /// Quality-of-Life components, like **Quark**.
    Qol,
    /// Stuff that expands the game's storage systems, like **Expanded Storage**.
    Storage,
    /// Stuff that introduces technology to the game, like **Create** or **AE2**.
    Technology,
    /// Stuff that improves the game's visuals, like **Euphoria Patches** or **Wakes**.
    Visual,
    /// Stuff that adds new wildlife to the game, like **Alex's Mobs**.
    Wildlife,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagInformation {
    pub main: Option<Tag>,
    pub others: Vec<Tag>,
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
    pub(crate) tags: TagInformation,
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
    ///
    /// # Panics
    ///
    /// This function will panic if the [parent](std::path::Path::parent) of this [`Component`]'s
    /// [local storage path](Self::local_storage_path) ends up being [`None`], which shouldn't happen.
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
    /// 2. Filter the versions based on the `loaders` and `game_versions` fields.
    /// 3. Pick the latest from the compatible ones.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - It fails to query the Modrinth API;
    /// - None of the versions of the component are compatible with the provided [`Instance`];
    /// - There are no URLs to where the component's file can be downloaded (unlikely...)
    #[tracing::instrument]
    pub fn fetch_from_modrinth(slug: &str, instance: &Instance) -> Result<Self, AddError> {
        let metadata_url = format!("https://api.modrinth.com/v2/project/{slug}");
        let versions_url = format!("https://api.modrinth.com/v2/project/{slug}/version");
        let metadata: modrinth::Metadata = reqwest::blocking::get(metadata_url)?.json()?;
        let mut versions: Vec<modrinth::Version> = reqwest::blocking::get(versions_url)?.json()?;

        // Only leave versions that are both loader- and version-compatible with the instance.
        versions.retain(|v| {
            // Resourcepacks and shaders may be loaded even if they are made for a different version.
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
        let main_tag = self::pick_main_tag()?;
        let other_tags = self::pick_secondary_tags(&main_tag)?;
        let component = Self {
            slug: slug.to_owned(),
            category: metadata.category,
            tags: TagInformation {
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

fn pick_main_tag() -> Result<Option<Tag>, AddError> {
    let main_tag: Option<Tag> = {
        let message = "Choose the main tag for this component:";
        let options = Tag::iter()
            .filter(|tag| !matches!(tag, Tag::Custom(_)))
            .collect();
        match inquire::Select::new(message, options)
            .with_page_size(Tag::iter().count())
            .with_help_message("Skip with [Escape] to provide a custom tag")
            .prompt_skippable()?
        {
            tag @ Some(_) => tag,
            None => {
                let message = "Provide a custom tag for this component:";
                inquire::Text::new(message)
                    .prompt_skippable()?
                    .map(|tag| tag.trim().to_lowercase())
                    .map(Tag::Custom)
            }
        }
    };
    Ok(main_tag)
}

fn pick_secondary_tags(main_tag: &Option<Tag>) -> Result<Vec<Tag>, AddError> {
    let other_tags: Vec<Tag> = {
        let message = "Add some additional tags for this component?";
        let options = Tag::iter()
            .filter(|tag| !matches!(tag, Tag::Custom(_)) && main_tag.as_ref() != Some(tag))
            .collect();
        inquire::MultiSelect::new(message, options)
            .with_page_size(Tag::iter().count())
            .with_help_message("This step can be freely skipped.")
            .prompt_skippable()?
            .unwrap_or_default()
    };
    Ok(other_tags)
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

pub(crate) mod modrinth {
    use super::Category;
    use crate::index::file::{Hashes, Requirement};
    use crate::instance::Loader;
    use color_eyre::owo_colors::OwoColorize;
    use serde::Deserialize;
    use std::fmt;
    use url::Url;

    #[derive(Deserialize, Debug)]
    pub struct File {
        pub hashes: Hashes,
        pub url: Url,
        pub filename: String,
        pub size: usize,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    pub struct Metadata {
        #[serde(rename = "project_type")]
        pub category: Category,
        pub client_side: Requirement,
        pub server_side: Requirement,
    }

    #[derive(Deserialize, Debug)]
    pub struct Version {
        pub id: String,
        pub name: String,
        pub game_versions: Vec<String>,
        pub loaders: Vec<Loader>,
        pub date_published: chrono::DateTime<chrono::Utc>,
        pub files: Vec<File>,
    }

    impl fmt::Display for Version {
        fn fmt(&self, stream: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                stream,
                "{name} [ID: {id}] - Supported loaders: {loaders:?}, released: {date}",
                name = self.name.yellow().bold(),
                id = self.id.bold(),
                loaders = self.loaders.bright_red(),
                date = self.date_published.format("%b %e, %Y").bright_blue().bold()
            )?;
            Ok(())
        }
    }
}

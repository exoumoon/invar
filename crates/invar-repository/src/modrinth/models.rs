use std::fmt;

use chrono::{DateTime, Utc};
use invar_component::{Category, Hashes, Requirement};
use invar_pack::instance::Loader;
use invar_pack::instance::version::MinecraftVersion;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(rename = "project_types")]
    pub types: Vec<Category>,
    pub game_versions: Vec<MinecraftVersion>,
    pub loaders: Vec<Loader>,
    pub versions: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub date_published: DateTime<Utc>,
    pub files: Vec<File>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub hashes: Hashes,
    pub url: Url,
    #[serde(rename = "filename")]
    pub name: String,
    pub size: usize,
}

#[derive(Deserialize, Debug)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: String,
    pub file_name: Option<String>,
    pub dependency_type: Requirement,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Metadata {
    pub project_type: Category,
    pub client_side: Requirement,
    pub server_side: Requirement,
}

impl fmt::Display for Version {
    fn fmt(&self, stream: &mut fmt::Formatter<'_>) -> fmt::Result {
        use color_eyre::owo_colors::OwoColorize;
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

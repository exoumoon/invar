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

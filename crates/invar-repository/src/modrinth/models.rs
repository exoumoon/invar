use std::collections::HashSet;
use std::fmt;

use chrono::{DateTime, Utc};
use color_eyre::owo_colors::OwoColorize;
use invar_component::{Category, Hashes, Requirement};
use invar_pack::instance::version::MinecraftVersion;
use invar_pack::instance::{Instance, Loader};
use serde::Deserialize;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: Option<String>,
    #[serde(rename = "project_types")]
    pub types: HashSet<Category>,
    pub game_versions: HashSet<MinecraftVersion>,
    pub loaders: HashSet<Loader>,
    pub versions: HashSet<String>,
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub project_types: HashSet<Category>,
    pub game_versions: HashSet<String>,
    pub loaders: HashSet<Loader>,
    pub date_published: DateTime<Utc>,
    pub environment: Option<Environment>,
    pub files: Vec<File>,
    pub dependencies: Vec<Dependency>,
}

impl Version {
    #[must_use]
    pub fn is_compatible(&self, instance: &Instance) -> bool {
        let version_agnostic_project_types =
            HashSet::from([Category::Resourcepack, Category::Shader]);
        let is_version_agnostic = self
            .project_types
            .intersection(&version_agnostic_project_types)
            .count()
            >= 1;
        let is_for_correct_version = self
            .game_versions
            .contains(&instance.minecraft_version.to_string());
        let version_loaders: HashSet<Loader> = self.loaders.iter().copied().collect();
        let has_unknown_loader = self.loaders.contains(&Loader::Other);
        let has_supported_loader = instance
            .allowed_loaders()
            .intersection(&version_loaders)
            .count()
            >= 1;
        (is_version_agnostic || is_for_correct_version)
            && (has_unknown_loader || has_supported_loader)
    }

    pub fn required_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|dependency| dependency.dependency_type == Requirement::Required)
    }

    pub fn optional_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|dependency| dependency.dependency_type == Requirement::Optional)
    }
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

#[derive(Deserialize, Clone, Debug)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: Requirement,
    pub display_name: Option<String>,
    pub summary: Option<String>,
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(display_name) = &self.display_name {
            write!(formatter, "{} ", display_name.purple())?;
        }

        write!(
            formatter,
            "[{id}]",
            id = self
                .project_id
                .as_ref()
                .map_or("Unknown", |project_id| project_id.as_str())
                .underline()
        )?;

        if let Some(summary) = &self.summary {
            let summary_cutoff = format!("{}...", summary.split_at(summary.len().min(80)).0);
            write!(formatter, " {}", summary_cutoff.bright_black())?;
        }

        Ok(())
    }
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    ClientOnly,
    ServerOnly,
    #[serde(other)]
    ClientAndServer,
}

impl From<Environment> for invar_component::Env {
    fn from(environment: Environment) -> Self {
        match environment {
            Environment::ClientOnly => Self {
                client: Requirement::Required,
                server: Requirement::Unsupported,
            },

            Environment::ServerOnly => Self {
                client: Requirement::Unsupported,
                server: Requirement::Required,
            },

            Environment::ClientAndServer => Self {
                client: Requirement::Required,
                server: Requirement::Required,
            },
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, stream: &mut fmt::Formatter<'_>) -> fmt::Result {
        use color_eyre::owo_colors::OwoColorize;
        write!(
            stream,
            "{name} [ID: {id}] - Supported loaders: {loaders:?}, released: {date}",
            name = self.name.purple().bold(),
            id = self.id.bold(),
            loaders = self.loaders.blue(),
            date = self.date_published.format("%b %e, %Y").cyan().bold()
        )?;
        Ok(())
    }
}

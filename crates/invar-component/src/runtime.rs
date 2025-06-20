use std::path::PathBuf;

use crate::{Category, Component};

#[derive(strum::Display, Clone, Copy, PartialEq, Eq, Debug)]
#[strum(serialize_all = "lowercase")]
#[must_use]
pub enum RuntimeDirectory {
    Config,
    Datapacks,
    Mods,
    Resourcepacks,
    Shaderpacks,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct RuntimePath {
    directory: RuntimeDirectory,
    filename: PathBuf,
}

impl From<Category> for RuntimeDirectory {
    fn from(category: Category) -> Self {
        match category {
            Category::Mod => Self::Mods,
            Category::Resourcepack => Self::Resourcepacks,
            Category::Shader => Self::Shaderpacks,
            Category::Datapack => Self::Datapacks,
            Category::Config => Self::Config,
        }
    }
}

impl Component {
    pub fn runtime_path(&self) -> RuntimePath {
        RuntimePath {
            directory: self.category.into(),
            filename: self.source.file_name(),
        }
    }
}

impl From<RuntimeDirectory> for PathBuf {
    fn from(runtime_directory: RuntimeDirectory) -> Self {
        Self::from(runtime_directory.to_string())
    }
}

impl From<RuntimePath> for PathBuf {
    fn from(runtime_path: RuntimePath) -> Self {
        let RuntimePath {
            directory,
            filename,
        } = runtime_path;

        let mut path = Self::from(directory);
        path.push(filename);

        path
    }
}

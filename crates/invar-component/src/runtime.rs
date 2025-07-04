use std::path::PathBuf;

use strum::{Display, EnumIter, EnumString};

use crate::{Category, Component};

#[derive(Display, EnumIter, EnumString, Clone, Copy, PartialEq, Eq, Debug)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
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

impl From<RuntimeDirectory> for Category {
    fn from(runtime_dir: RuntimeDirectory) -> Self {
        match runtime_dir {
            RuntimeDirectory::Config => Self::Config,
            RuntimeDirectory::Datapacks => Self::Datapack,
            RuntimeDirectory::Mods => Self::Mod,
            RuntimeDirectory::Resourcepacks => Self::Resourcepack,
            RuntimeDirectory::Shaderpacks => Self::Shader,
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

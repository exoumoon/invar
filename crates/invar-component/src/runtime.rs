use std::ffi::OsStr;
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

impl RuntimePath {
    pub const fn new(directory: RuntimeDirectory, filename: PathBuf) -> Self {
        Self {
            directory,
            filename,
        }
    }
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
        let directory = RuntimeDirectory::from(self.category);

        let source_file_name = self.source.file_name();
        let id_only_name = format!(
            "{id}.{extension}",
            id = self.id,
            extension = source_file_name
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("zip"),
        );

        let filename = match self.category {
            Category::Mod | Category::Datapack | Category::Config => source_file_name,
            Category::Resourcepack | Category::Shader => PathBuf::from(id_only_name),
        };

        RuntimePath::new(directory, filename)
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

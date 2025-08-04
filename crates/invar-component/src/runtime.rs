use std::ffi::OsStr;
use std::path::PathBuf;

use strum::{Display, EnumIter, EnumString};

use crate::{Category, Component, Source};

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
    directory: Option<RuntimeDirectory>,
    filename: PathBuf,
}

impl RuntimePath {
    pub const fn new(directory: RuntimeDirectory, filename: PathBuf) -> Self {
        Self {
            directory: Some(directory),
            filename,
        }
    }

    pub const fn new_root(filename: PathBuf) -> Self {
        Self {
            directory: None,
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
        match &self.source {
            Source::Local(local_component) => match &local_component.entry.runtime_path {
                Some(runtime_path_override) => RuntimePath::new_root(runtime_path_override.clone()),
                None => RuntimePath::new(directory, local_component.entry.uncategorized_path()),
            },

            Source::Remote(remote_component) => match self.category {
                Category::Mod | Category::Datapack | Category::Config => {
                    RuntimePath::new(directory, remote_component.file_name.clone())
                }

                Category::Resourcepack | Category::Shader => {
                    let file_extension = remote_component
                        .file_name
                        .extension()
                        .and_then(OsStr::to_str)
                        .map_or("zip", Into::into);
                    let filename = format!("{id}.{file_extension}", id = self.id);
                    RuntimePath::new(directory, PathBuf::from(filename))
                }
            },
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

        match directory {
            None => filename,
            Some(directory) => {
                let mut path = Self::from(directory);
                path.push(filename);
                path
            }
        }
    }
}

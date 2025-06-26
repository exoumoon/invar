use std::path::{Path, PathBuf};
use std::{fs, io};

use invar_component::{
    Component, Env, LocalComponent, LocalComponentEntry, Requirement, RuntimeDirectory, Source,
    TagInformation,
};
use invar_pack::Pack;
use persist::PersistedEntity;
use strum::IntoEnumIterator;
use walkdir::WalkDir;

pub mod persist;

#[derive(Debug, Clone)]
pub struct LocalRepository {
    root_directory: PathBuf,
    pub pack: Pack,
}

impl LocalRepository {
    pub const COMPONENT_FILE_EXTENSION: &str = "yml";
    pub const COMPONENT_FILE_SUFFIX: &str = "invar";

    pub const BACKUP_DIRECTORY: &str = ".backups";
    pub const BACKUP_DIRECTORY_SEP: char = '_';

    /// "Open" a local repository in `root_directory`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the underlying `root_directory`
    /// can't be found or canonicalized. See [`std::fs::canonicalize`] for more
    /// information on that.
    pub fn open(root_directory: impl AsRef<Path>) -> Result<Self, self::Error> {
        let root_directory = root_directory.as_ref().canonicalize()?;
        std::env::set_current_dir(&root_directory)?;
        let pack = Pack::read()?;
        Ok(Self {
            root_directory,
            pack,
        })
    }

    /// "Open" a local repository in the root of the current `git` repo.
    ///
    /// # Errors
    ///
    /// This function will return an error if the current directory is not a
    /// part of a `git` repo, or in any cases described in [`Self::open`].
    pub fn open_at_git_root() -> Result<Self, self::Error> {
        let cwd = std::env::current_dir()?;
        let mut root_directory = cwd.canonicalize()?;
        while !root_directory.read_dir()?.flatten().any(|dir| {
            const GIT_SIGNATURE: &str = ".git";
            let is_git = dir.file_name() == GIT_SIGNATURE;
            let is_dir = dir.file_type().is_ok_and(|file_type| file_type.is_dir());
            is_git && is_dir
        }) {
            let message = "Failed to find a Git repository in $PWD or its ancestors";
            root_directory = root_directory
                .parent()
                .ok_or(io::Error::new(io::ErrorKind::NotFound, message))?
                .to_path_buf();
        }

        Self::open(root_directory)
    }

    /// Returns the list of components of this [`LocalStorage`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the underlying data directories
    /// cannot be traversed, or files inside of them cannot be read. It will
    /// also return an error if a metadata file does not contain a valid
    /// [`Component`] schema.
    pub fn components(&self) -> Result<Vec<Component>, self::Error> {
        let mut components = vec![];

        for entry in WalkDir::new(&self.root_directory).into_iter().flatten() {
            if entry.is_component_file() {
                let yml = std::fs::read_to_string(entry.path())?;
                let component: Component = serde_yml::from_str(&yml)?;
                components.push(component);
            }
        }

        for entry @ LocalComponentEntry { path, category } in &self.pack.local_components {
            let component = Component {
                id: entry.id(),
                category: *category,
                tags: TagInformation::default(),
                environment: Env {
                    client: Requirement::Required,
                    server: Requirement::Required,
                },
                source: Source::Local(LocalComponent { path: path.clone() }),
            };

            components.push(component);
        }

        Ok(components)
    }

    /// Returns the path where this component should be saved.
    ///
    /// **Note:** If the component has a "main" tag associated with it, we'll
    /// store it in a subfolder with all the other components that share
    /// this "main" tag. However, as this function is only really used when
    /// choosing where to save a component when it's first being added, this
    /// **main tag <-> subfolder** relationship does not affect reading and
    /// later usage of a component, so the end user can freely shuffle
    /// components around and edit tags without worrying about keeping that
    /// relationship intact.
    #[must_use]
    pub fn component_path(&self, component: &Component) -> PathBuf {
        let mut path = self.root_directory.clone();

        let runtime_dir = RuntimeDirectory::from(component.category).to_string();
        path.push(runtime_dir);

        if let Some(main_tag) = &component.tags.main {
            path.push(main_tag.to_string());
        }

        path.push(format!(
            "{id}.{sfx}.{ext}",
            id = component.id,
            sfx = Self::COMPONENT_FILE_SUFFIX,
            ext = Self::COMPONENT_FILE_EXTENSION
        ));

        path
    }

    pub fn save_component(&mut self, component: &Component) -> Result<(), self::Error> {
        match component.source {
            Source::Local(ref source) => {
                self.pack.local_components.push(LocalComponentEntry {
                    path: source.path.clone(),
                    category: component.category,
                });
                self.pack.write()?;
            }
            Source::Remote(_) => {
                let target_path = self.component_path(component);
                let yaml_repr = serde_yml::to_string(component)?;
                fs::write(target_path, yaml_repr)?;
            }
        }

        Ok(())
    }

    pub fn remove_component<S>(&mut self, id: S) -> Result<(), self::Error>
    where
        S: AsRef<str>,
    {
        for component in self
            .components()?
            .into_iter()
            .filter(|component| component.id == id.as_ref().into())
        {
            if component.is_remote() {
                let path_to_remove = self.component_path(&component);
                std::fs::remove_file(path_to_remove)?;
            } else {
                self.pack
                    .local_components
                    .retain(|local_entry| local_entry.id() != id.as_ref().into());
                self.pack.write()?;
            }
        }

        Ok(())
    }

    /// Create the data subdirectories in the current directory.
    ///
    /// # Errors
    ///
    /// May return an I/O error if some folders are already set up.
    pub fn setup_directories(&self) -> io::Result<()> {
        for directory in RuntimeDirectory::iter() {
            let mut target = self.root_directory.clone();
            target.push(PathBuf::from(directory));
            if !fs::exists(&target)? {
                fs::create_dir_all(&target)?;
            }
            target.push(".gitkeep");
            if !fs::exists(&target)? {
                let _ = fs::File::create(&target)?;
            }
        }

        fs::create_dir_all(Self::BACKUP_DIRECTORY)?;
        fs::write(format!("{}/.gitignore", Self::BACKUP_DIRECTORY), "*\n")?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error {
    Io(#[from] io::Error),
    SerdeYml(#[from] serde_yml::Error),
    #[error("A component file does not have a name")]
    EmptyFilename,
    #[error("Failed to read from or write to a persistent file")]
    Persistence(#[from] persist::PersistError),
}

trait ComponentFile {
    fn is_component_file(&self) -> bool;
}

impl ComponentFile for std::fs::DirEntry {
    fn is_component_file(&self) -> bool {
        let is_file = self.file_type().is_ok_and(|ft| ft.is_file());
        let is_metadata = self.path().to_string_lossy().ends_with(&format!(
            ".{}.{}",
            LocalRepository::COMPONENT_FILE_SUFFIX,
            LocalRepository::COMPONENT_FILE_EXTENSION
        ));
        is_file && is_metadata
    }
}

impl ComponentFile for walkdir::DirEntry {
    fn is_component_file(&self) -> bool {
        let is_file = self.file_type().is_file();
        let is_metadata = self.path().to_string_lossy().ends_with(&format!(
            ".{}.{}",
            LocalRepository::COMPONENT_FILE_SUFFIX,
            LocalRepository::COMPONENT_FILE_EXTENSION
        ));
        is_file && is_metadata
    }
}

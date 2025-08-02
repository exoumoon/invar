pub mod persist;

use std::path::{Path, PathBuf};
use std::{fs, io};

use invar_component::{
    Component, Env, LocalComponent, LocalComponentEntry, Requirement, RuntimeDirectory, Source,
    TagInformation,
};
use invar_pack::Pack;
use invar_pack::settings::VcsMode;
use persist::PersistedEntity;
use strum::IntoEnumIterator;
use walkdir::WalkDir;

pub struct LocalRepository {
    pub root_directory: PathBuf,
    pub pack: Pack,
    pub git_repository: git2::Repository,
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
        let git_repository = git2::Repository::open(&root_directory)?;
        std::env::set_current_dir(&root_directory)?;
        let pack = Pack::read()?;
        Ok(Self {
            root_directory,
            pack,
            git_repository,
        })
    }

    /// "Open" a local repository in the root of the current `git` repo.
    ///
    /// # Errors
    ///
    /// This function will return an error if the current directory is not a
    /// part of a `git` repo, or in any cases described in [`Self::open`].
    #[expect(clippy::missing_panics_doc, reason = "expect")]
    pub fn open_at_git_root() -> Result<Self, self::Error> {
        let cwd = std::env::current_dir()?;
        let git_repository = git2::Repository::discover(cwd)?;
        let git_root = git_repository
            .path()
            .parent()
            .expect("The .git dir must have a parent");

        let base_repository = Self::open(git_root)?;
        let local_repository = Self {
            git_repository,
            ..base_repository
        };

        Ok(local_repository)
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

    pub fn remove_components<S>(&mut self, id: S) -> Result<(), self::Error>
    where
        S: AsRef<str>,
    {
        let mut result = Err(Error::ComponentNotFound);

        for component in self
            .components()?
            .into_iter()
            .filter(|component| component.id == id.as_ref().into())
        {
            if component.source.is_remote() {
                let path_to_remove = self.component_path(&component);
                std::fs::remove_file(path_to_remove)?;
            } else {
                self.pack
                    .local_components
                    .retain(|local_entry| local_entry.id() != id.as_ref().into());
                self.pack.write()?;
            }

            result = Ok(());
        }

        result
    }

    pub fn setup(&self) -> Result<(), self::Error> {
        let git_repo = git2::Repository::init(".")?;
        fs::create_dir_all(Self::BACKUP_DIRECTORY)?;
        fs::write(format!("{}/.gitignore", Self::BACKUP_DIRECTORY), "*\n")?;

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

        match self.pack.settings.vcs_mode {
            VcsMode::Manual => { /* do nothing */ }
            VcsMode::TrackComponents => {
                let mut index = git_repo.index()?;
                index.add_all(std::iter::once("*"), git2::IndexAddOption::DEFAULT, None)?;
                index.write()?; // NOTE: This is essentially `git add *`.

                let signature = git_repo.signature()?;
                let tree_oid = git_repo.index()?.write_tree()?;
                let tree = git_repo.find_tree(tree_oid)?;

                let commit_parents = &[];
                let commit_message = "invar: Initial commit";
                git_repo.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    commit_message,
                    &tree,
                    commit_parents,
                )?;
            }
        }

        Ok(())
    }

    pub fn modpack_file_name(&self) -> Result<PathBuf, git2::Error> {
        let current_local_time = chrono::Local::now().format("%Y%m%d-%H%M");
        let commit_hash = self.git_repository.head()?.peel_to_commit()?.id();
        let modpack_file_name = format!(
            "{pack_name}-v{pack_version}-{current_local_time}-{commit_hash}.mrpack",
            pack_name = self.pack.name,
            pack_version = self.pack.version,
            commit_hash = &commit_hash.to_string()[..7],
        );

        Ok(PathBuf::from(modpack_file_name))
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
    #[error("No matching components found")]
    ComponentNotFound,
    #[error("Failed to interact with the underlying Git repository")]
    Git(#[from] git2::Error),
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

use crate::component::Component;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::instrument;
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, self::Error>;

/// Possible errors that may arise while interacting with local storage.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("An I/O error occurred, faulty path: {faulty_path:?}")]
    Io {
        source: io::Error,
        faulty_path: Option<PathBuf>,
    },

    #[error(transparent)]
    SerdeYml(#[from] serde_yml::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),
}

/// A trait that represents an entity (type) that can be persisted in a file.
pub trait PersistedEntity: Serialize + for<'de> Deserialize<'de> {
    /// The path to the file where this entity should be persisted.
    const FILE_PATH: &'static str;

    /// Deserializes an instance of [`Self`] from [`Self::FILE_PATH`].
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error reading
    /// [`Self::FILE_PATH`] or an error occurs when deserializing its
    /// contents into [`Self`].
    #[instrument]
    fn read() -> Result<Self> {
        let path = find_and_expand(Path::new(Self::FILE_PATH))?;
        let yaml = fs::read_to_string(&path).map_err(|source| Error::Io {
            source,
            faulty_path: Some(path.clone()),
        })?;
        let entity = serde_yml::from_str(&yaml)?;
        Ok(entity)
    }

    /// Serialize `self` into a string and write it to [`Self::FILE_PATH`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an error occurs while serializing
    /// [`self`](Self) to a string or while writing that string to
    /// [`Self::FILE_PATH`].
    #[must_use = "You haven't checked if the entity was successfully persisted"]
    #[instrument(skip(self))]
    fn write(&self) -> Result<()> {
        let path = PathBuf::from(Self::FILE_PATH);
        let yaml = serde_yml::to_string(self)?;
        fs::write(&path, yaml).map_err(|source| Error::Io {
            source,
            faulty_path: Some(path.clone()),
        })?;
        Ok(())
    }
}

/// Iterate over all metadata files in local storage.
///
/// # Errors
///
/// This function will return an error if errors occur in the
/// filesystem iterator produced by the [`walkdir`] crate.
pub fn metadata_files<P>(path: P) -> Result<impl Iterator<Item = walkdir::DirEntry>>
where
    P: AsRef<Path>,
{
    let iterator = WalkDir::new(path.as_ref())
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|file| file.file_type().is_file())
        .filter(|file| {
            file.path()
                .to_str()
                .is_some_and(|path| path.ends_with(Component::LOCAL_STORAGE_SUFFIX))
        });

    Ok(iterator)
}

/// Synchronize cached writes to persistent storage.
///
/// # Errors
///
/// This function will return an error if the `sync` command fails.
pub fn try_sync() -> Result<()> {
    match std::process::Command::new("sync").status() {
        Ok(_) => Ok(()),
        Err(source) => Err(Error::Io {
            source,
            faulty_path: None,
        }),
    }
}

// NOTE: A shorthand for `expanding` a path and logging an error if one arises
// in the process.
fn find_and_expand(path: &Path) -> Result<PathBuf> {
    path.canonicalize().map_err(|source| Error::Io {
        source,
        faulty_path: Some(path.to_path_buf()),
    })
}

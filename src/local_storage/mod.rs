use crate::component::Component;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::{error, instrument};
use walkdir::{DirEntry, WalkDir};

/// Possible errors that may arise while interacting with local storage.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("An I/O error occurred")]
    Io {
        #[from]
        source: io::Error,
    },

    #[error(transparent)]
    Serde {
        #[from]
        source: serde_yml::Error,
    },

    #[error(transparent)]
    Walkdir {
        #[from]
        source: walkdir::Error,
    },
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
    fn read() -> Result<Self, Error> {
        let path = find_and_expand(Path::new(Self::FILE_PATH))?;
        let yaml =
            fs::read_to_string(&path).inspect_err(|_| error!(?path, "failed to read file"))?;
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
    fn write(&self) -> Result<(), Error> {
        let path = PathBuf::from(Self::FILE_PATH);
        let yaml = serde_yml::to_string(self)?;
        fs::write(&path, yaml).inspect_err(|_| error!(target = ?path, "failed to write file"))?;
        Ok(())
    }
}

fn find_and_expand(path: &Path) -> Result<PathBuf, Error> {
    Ok(path
        .canonicalize()
        .inspect_err(|_| error!(?path, "failed to locate file"))?)
}

/// Iterate over all metadata files in local storage.
///
/// # Errors
///
/// This function will return an error if errors occur in the
/// filesystem iterator produced by the [`walkdir`] crate.
pub fn metadata_files<S>(subdir: S) -> Result<impl Iterator<Item = DirEntry>, walkdir::Error>
where
    S: AsRef<str>,
{
    let iterator = WalkDir::new(subdir.as_ref())
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|file| file.file_type().is_file())
        .filter(|file| {
            file.path()
                .to_str()
                .is_some_and(|path| path.ends_with(Component::LOCAL_STORAGE_SUFFIX))
        });

    Ok(iterator)
}

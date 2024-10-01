use crate::component::Component;
use serde::{Deserialize, Serialize};
use std::{fs, io};
use walkdir::{DirEntry, WalkDir};

/// Possible errors that may be encountered while interacting with a persistent local storage.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("An I/O error occurred: {0:?}")]
    Io(#[from] io::Error),

    #[error("A (de)serialization error occurred: {0:?}")]
    Serde(#[from] serde_yml::Error),

    #[error("Directory traversal error: {0:?}")]
    Walkdir(#[from] walkdir::Error),
}

/// A trait that represents an entity (type) that can be persisted in a file.
pub trait PersistedEntity: Serialize + for<'de> Deserialize<'de> {
    /// The path to the file where this entity should be persisted.
    const FILE_PATH: &'static str;

    /// Deserializes an instance of [`Self`] from a the contents of [`Self::FILE_PATH`].
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error reading [`Self::FILE_PATH`]
    /// or an error occurs when deserializing its contents into [`Self`].
    #[tracing::instrument(name = "ls_read")]
    fn read() -> Result<Self, Error> {
        tracing::debug!(file = ?Self::FILE_PATH, "reading local storage");
        let yaml = fs::read_to_string(Self::FILE_PATH)?;
        Ok(serde_yml::from_str(&yaml)?)
    }

    /// Serialize [`self`](Self) into a string and write it to [`Self::FILE_PATH`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an error occurs while serializing [`self`](Self)
    /// to a string or while writing that string to [`Self::FILE_PATH`].
    #[must_use = "You haven't checked if the entity was successfully persisted"]
    #[tracing::instrument(name = "ls_write", skip(self))]
    fn write(&self) -> Result<(), Error> {
        tracing::debug!(file = ?Self::FILE_PATH, "writing to local storage");
        let yaml = serde_yml::to_string(self)?;
        Ok(fs::write(Self::FILE_PATH, yaml)?)
    }
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

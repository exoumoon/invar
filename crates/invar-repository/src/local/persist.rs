use std::path::{Path, PathBuf};
use std::{fs, io};

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
#[must_use]
pub enum PersistError {
    #[error("An I/O error occurred, path at fault: {path:?}")]
    Io {
        source: io::Error,
        path: Option<PathBuf>,
    },

    #[error("Failed to (de)serialize data to/from YAML")]
    SerdeYml(#[from] serde_yml::Error),
}

impl PersistError {
    pub const fn io(source: io::Error, path: PathBuf) -> Self {
        Self::Io {
            source,
            path: Some(path),
        }
    }
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
    fn read() -> Result<Self, PersistError> {
        let path = Path::new(Self::FILE_PATH)
            .canonicalize()
            .map_err(|source| PersistError::io(source, PathBuf::from(Self::FILE_PATH)))?;
        let yml = fs::read_to_string(&path).map_err(|source| PersistError::io(source, path))?;
        let entity = serde_yml::from_str(&yml)?;
        Ok(entity)
    }

    /// Serialize `self` into a string and write it to [`Self::FILE_PATH`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an error occurs while serializing
    /// [`self`](Self) to a string or while writing that string to
    /// [`Self::FILE_PATH`].
    fn write(&self) -> Result<(), PersistError> {
        let path = PathBuf::from(Self::FILE_PATH);
        let yml = serde_yml::to_string(self)?;
        fs::write(&path, yml).map_err(|source| PersistError::io(source, path))?;
        Ok(())
    }
}

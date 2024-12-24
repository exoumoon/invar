use crate::local_storage::PersistedEntity;
use crate::server::docker_compose;
use crate::{local_storage, BackupMode, Pack};
use chrono::{DateTime, Local};
use color_eyre::owo_colors::OwoColorize;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, fs};

pub const BACKUP_FOLDER: &str = ".backups";
pub const BACKUP_FOLDER_SEP: char = '_';
pub const GC_DELAY: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Path to the directory where the backup lives.
    pub path: PathBuf,
    /// The sequential number of the backup.
    pub seq_number: usize,
    /// When this backup was created.
    pub created_at: DateTime<Local>,
}

/// Load all backups found in `local_storage`.
///
/// # Errors
///
/// See [`local_storage::Error`] for possible error causes.
pub fn get_all_backups() -> local_storage::Result<Vec<Backup>> {
    let backups = fs::read_dir(BACKUP_FOLDER)
        .map_err(|source| local_storage::Error::Io {
            source,
            faulty_path: Some(PathBuf::from(BACKUP_FOLDER)),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| local_storage::Error::Io {
            source,
            faulty_path: Some(PathBuf::from(BACKUP_FOLDER)),
        })?
        .into_iter()
        .filter(|folder| {
            folder
                .metadata()
                .ok()
                .map(|md| md.is_dir())
                .is_some_and(|id_dir| id_dir)
        })
        .map(|folder| -> Result<_, local_storage::Error> {
            let seq_number = folder
                .path()
                .file_name()
                .and_then(|folder_name| {
                    folder_name
                        .to_string_lossy()
                        .split(BACKUP_FOLDER_SEP)
                        .next()
                        .and_then(|marker| marker.parse::<usize>().ok())
                })
                .unwrap_or(usize::MAX);
            let created_at = folder
                .path()
                .file_name()
                .and_then(|folder_name| {
                    folder_name
                        .to_string_lossy()
                        .split(BACKUP_FOLDER_SEP)
                        .last()
                        .and_then(|marker| marker.parse::<DateTime<Local>>().ok())
                })
                .unwrap_or(DateTime::UNIX_EPOCH.into());
            Ok(Backup {
                seq_number,
                created_at,
                path: folder
                    .path()
                    .canonicalize()
                    .map_err(|source| local_storage::Error::Io {
                        source,
                        faulty_path: Some(folder.path()),
                    })?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sorted_unstable_by_key(|folder| folder.seq_number)
        .rev()
        .collect_vec();
    Ok(backups)
}

/// Create a new [`Backup`].
///
/// # Errors
///
/// See [`local_storage::Error`] for possible error causes.
pub fn create_new(tag: Option<&str>) -> Result<Backup, self::Error> {
    let pack_name = Pack::read()?.name;
    let seq_number = get_all_backups()?
        .into_iter()
        .map(|backup| backup.seq_number)
        .sorted_unstable()
        .last()
        .unwrap_or_default()
        + 1;
    let created_at = Local::now();
    let target_dir = format!(
        "{BACKUP_FOLDER}/{seq_number}_{pack_name}{tag}_{created_at}",
        tag = tag.map(|tag| format!("({tag})")).unwrap_or_default(),
    );
    match copy_dir::copy_dir(docker_compose::DATA_VOLUME_PATH, &target_dir) {
        Err(source) => {
            return Err(local_storage::Error::Io {
                source,
                faulty_path: Some(target_dir.into()),
            }
            .into())
        }
        Ok(error_list) if !error_list.is_empty() => return Err(Error::CopyDir { error_list }),
        Ok(_) => {}
    };

    Ok(Backup {
        path: target_dir.into(),
        seq_number,
        created_at,
    })
}

/// Remove backups that are old enough to be removed.
///
/// # Errors
///
/// See [`local_storage::Error`] for possible error causes.
pub fn gc() -> Result<GcResult, self::Error> {
    match local_storage::try_sync() {
        Ok(()) => {}
        Err(error) => {
            tracing::warn!(%error, "Failed to `sync` before garbage-collecting backups");
            tracing::warn!("Waiting for {GC_DELAY:?} as a measure of protection...");
            std::thread::sleep(GC_DELAY);
        }
    }

    let mut all_backups = get_all_backups()?;
    match Pack::read()?.settings.backup_mode {
        BackupMode::StartStop { min_depth } => {
            let remaining = all_backups.drain(..min_depth).collect_vec();
            let removed = all_backups;
            for old_backup in removed.iter().rev() {
                fs::remove_dir_all(&old_backup.path).map_err(|source| {
                    local_storage::Error::Io {
                        source,
                        faulty_path: Some(old_backup.path.clone()),
                    }
                })?;
            }
            return Ok(GcResult { removed, remaining });
        }
        BackupMode::Manual => {
            tracing::warn!("The pack's setting dictate manual backups. Doing nothing");
        }
    }

    Ok(GcResult {
        removed: vec![],
        remaining: all_backups,
    })
}

#[derive(Serialize, Clone, Debug)]
pub struct GcResult {
    pub removed: Vec<Backup>,
    pub remaining: Vec<Backup>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    LocalStorage(#[from] local_storage::Error),
    #[error("Errors occured while creating backup")]
    CopyDir { error_list: Vec<std::io::Error> },
}

impl fmt::Display for Backup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Backup {seq_number}, created at {created_at}, path: {path:?}",
            seq_number = format!("#{}", self.seq_number).bold().yellow(),
            created_at = self
                .created_at
                .format("%d/%m/%Y %H:%M:%S")
                .bold()
                .bright_yellow(),
            path = self.path.bold().blue(),
        )
    }
}

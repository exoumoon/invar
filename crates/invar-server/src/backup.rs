use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, fs};

use chrono::{DateTime, Local};
use color_eyre::owo_colors::OwoColorize;
use invar_pack::Pack;
use invar_pack::settings::BackupMode;
use invar_repository::LocalRepository;
use invar_repository::persist::PersistedEntity;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::docker_compose;

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
pub fn get_all_backups() -> Result<Vec<Backup>, std::io::Error> {
    let backups = fs::read_dir(LocalRepository::BACKUP_DIRECTORY)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|directory| {
            directory
                .metadata()
                .map(|metadata| metadata.is_dir())
                .is_ok_and(|is_dir| is_dir)
        })
        .map(|directory| -> Result<_, std::io::Error> {
            let seq_number = directory
                .path()
                .file_name()
                .and_then(|directory_name| {
                    directory_name
                        .to_string_lossy()
                        .split(LocalRepository::BACKUP_DIRECTORY_SEP)
                        .next()
                        .and_then(|marker| marker.parse::<usize>().ok())
                })
                .unwrap_or(usize::MAX);
            let created_at = directory
                .path()
                .file_name()
                .and_then(|directory_name| {
                    directory_name
                        .to_string_lossy()
                        .split(LocalRepository::BACKUP_DIRECTORY_SEP)
                        .next_back()
                        .and_then(|marker| marker.parse::<DateTime<Local>>().ok())
                })
                .unwrap_or(DateTime::UNIX_EPOCH.into());
            Ok(Backup {
                seq_number,
                created_at,
                path: directory.path().canonicalize()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sorted_unstable_by_key(|directory| directory.seq_number)
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
    // let pack_name = Pack::read()?.name;
    let pack_name = "fuckshit";
    let seq_number = get_all_backups()?
        .into_iter()
        .map(|backup| backup.seq_number)
        .sorted_unstable()
        .last()
        .unwrap_or_default()
        + 1;

    let created_at = Local::now();
    let target_dir = format!(
        "{directory}/{seq_number}_{pack_name}{tag}_{created_at}",
        directory = LocalRepository::BACKUP_DIRECTORY,
        tag = tag.map(|tag| format!("({tag})")).unwrap_or_default(),
    );

    match copy_dir::copy_dir(docker_compose::DATA_VOLUME_PATH, &target_dir) {
        Err(source) => return Err(source.into()),
        Ok(error_list) if !error_list.is_empty() => return Err(Error::CopyDir { error_list }),
        Ok(_) => {}
    }

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
    let mut all_backups = get_all_backups()?;
    match Pack::read().unwrap().settings.backup_mode {
        BackupMode::StartStop { min_depth } => {
            let remaining = all_backups.drain(..min_depth).collect_vec();
            let removed = all_backups;
            for old_backup in removed.iter().rev() {
                fs::remove_dir_all(&old_backup.path)?;
            }
            return Ok(GcResult { removed, remaining });
        }
        BackupMode::Manual => { /* The pack's setting dictate manual backups. Doing nothing */ }
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
    Io(#[from] std::io::Error),
    #[error("Failed to copy over files for backup")]
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

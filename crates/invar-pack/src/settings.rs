//! Per-pack configuration interface for **Invar**.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub vcs_mode: VcsMode,
    pub backup_mode: BackupMode,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcsMode {
    /// Auto-commit each added or removed component.
    #[default]
    TrackComponents,

    /// Initialize a Git repo upon pack setup, commit nothing automatically.
    Manual,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupMode {
    /// Create a backup before starting the server and after stopping it.
    ///
    /// This mode will keep `min_depth` most recent backups. When a new one is
    /// created and the count of backups exceeds `min_depth`, the oldest backups
    /// are deleted until there are exactly `min_depth` backups remaining.
    StartStop { min_depth: usize },

    /// Do not create or delete backups automatically.
    Manual,
}

impl Default for BackupMode {
    fn default() -> Self {
        Self::StartStop {
            // Pre-start and post-stop backups for the last 2 launches.
            min_depth: 4,
        }
    }
}

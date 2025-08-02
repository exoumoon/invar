//! Per-pack configuration interface for **Invar**.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub vcs_mode: VcsMode,
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

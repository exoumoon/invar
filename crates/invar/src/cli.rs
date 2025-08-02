use std::io;

use clap::builder::Styles;
use clap::builder::styling::AnsiColor::{BrightBlue, White, Yellow};
use clap::{Parser, ValueEnum};
use clap_complete::Generator;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete_nushell::Nushell;
use invar_component::Category;
use invar_pack::instance::Loader;
use semver::Version;

/// Styling for [`clap`]'s CLI interface.
const STYLES: Styles = Styles::styled()
    .usage(Yellow.on_default().bold())
    .literal(BrightBlue.on_default().bold())
    .placeholder(White.on_default().bold())
    .header(Yellow.on_default().bold());

#[derive(Parser, Debug)]
#[command(version, author, about, styles(STYLES))]
pub struct Options {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Manage the pack itself.
    Pack {
        #[command(subcommand)]
        action: PackAction,
    },

    /// Manage modpack's components.
    Component {
        #[command(subcommand)]
        action: ComponentAction,
    },

    /// Manage the self-hosted server.
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// Manage the local Invar repository.
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },

    /// Generate shell completions for this tool.
    Completions {
        /// Which shell to generate completions for.
        #[arg(short, long, value_enum)]
        shell: Shell,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum PackAction {
    /// Create a new pack in the current directory.
    #[clap(visible_alias("new"), visible_alias("create"))]
    Setup {
        /// The name of the created modpack.
        #[arg(short, long)]
        name: Option<String>,

        /// What game version to build upon.
        #[arg(long)]
        minecraft_version: Option<Version>,

        /// Which modloader to build upon.
        #[arg(short, long)]
        loader: Option<Loader>,

        /// Which loader version to use. Ignored if no loader is used.
        #[arg(long)]
        loader_version: Option<Version>,

        /// Don't ask for confirmation if there's already a pack in the current
        /// directory.
        #[arg(short, long)]
        overwrite: bool,
    },

    SetupDirectories,

    /// Read the local storage and show what Invar sees.
    Show,

    /// Export the modpack in `.mrpack` format.
    Export,
}

#[derive(clap::Subcommand, Debug)]
pub enum ComponentAction {
    /// Show the existing components in the pack.
    List,

    /// Add a new component to the pack.
    #[command(arg_required_else_help = true)]
    Add {
        /// The IDs of components to be added.
        ids: Vec<String>,

        /// Whether to tread `ids` as paths to local files.
        #[arg(short, long)]
        local: bool,

        /// Force all listed components to be added to this category.
        #[arg(short('c'), long("category"))]
        forced_category: Option<Category>,
    },

    /// Update one or more of the existing components.
    Update {
        /// The IDs of components to update (update all if not provided).
        ids: Vec<String>,
    },

    /// Remove one or more of the existing components.
    #[clap(visible_alias("delete"))]
    #[command(arg_required_else_help = true)]
    Remove {
        /// The IDs of components to remove.
        ids: Vec<String>,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ServerAction {
    /// Prepare for the first start of the server.
    Setup,

    /// Start the server, do nothing if it is already running.
    Start,

    /// Stop the server, do nothing if it is already stopped.
    Stop,

    /// Report the status of the server.
    Status,
}

#[derive(clap::Subcommand, Debug)]
pub enum RepoAction {
    /// Read the local repository and show what Invar sees.
    Show,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
#[expect(clippy::enum_variant_names, reason = "powershell for fuck's sake")]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
    Nushell,
}

impl Generator for Shell {
    fn file_name(&self, name: &str) -> String {
        match self {
            Self::Bash => Bash.file_name(name),
            Self::Elvish => Elvish.file_name(name),
            Self::Fish => Fish.file_name(name),
            Self::PowerShell => PowerShell.file_name(name),
            Self::Zsh => Zsh.file_name(name),
            Self::Nushell => Nushell.file_name(name),
        }
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn io::Write) {
        match self {
            Self::Bash => Bash.generate(cmd, buf),
            Self::Elvish => Elvish.generate(cmd, buf),
            Self::Fish => Fish.generate(cmd, buf),
            Self::PowerShell => PowerShell.generate(cmd, buf),
            Self::Zsh => Zsh.generate(cmd, buf),
            Self::Nushell => Nushell.generate(cmd, buf),
        }
    }
}

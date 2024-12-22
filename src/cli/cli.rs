use clap::builder::styling::AnsiColor::{BrightBlue, White, Yellow};
use clap::builder::Styles;
use clap::Parser;
use invar::instance::Loader;
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

    /// Read the local storage and show Invar sees.
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

        /// Show the component's metadata before writing it to disk.
        #[arg(short('d'), long("debug"))]
        show_metadata: bool,
    },

    /// Update one or more of the existing components.
    Update {
        /// The IDs of components to update (update all if not provided).
        slugs: Vec<String>,
    },

    /// Remove one or more of the existing components.
    #[clap(visible_alias("delete"))]
    #[command(arg_required_else_help = true)]
    Remove {
        /// The IDs of components to remove.
        slugs: Vec<String>,
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

    /// Manage backups of the server.
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum BackupAction {
    /// List out all the backups created in the past.
    List,

    /// Create a new backup at this point in time.
    Create,
}

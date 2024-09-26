use clap::builder::styling::AnsiColor::{BrightBlue, White, Yellow};
use clap::{builder::Styles, Parser};
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

        /// Don't ask for confirmation if there's already a pack in the current directory.
        #[arg(short, long)]
        overwrite: bool,
    },

    /// Read the local storage and show Invar sees.
    #[clap(visible_alias("debug"))]
    Show,

    /// Export the modpack in `.mrpack` format.
    Export,
}

#[derive(clap::Subcommand, Debug)]
pub enum ComponentAction {
    /// Show the existing components in the pack.
    List,

    /// Add a new component to the pack.
    Add {
        /// The ID of the component to be added.
        id: String,

        /// Where to get the component from. Inferred if left out.
        #[arg(short, long)]
        source: Option<ComponentSource>,
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

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum ComponentSource {
    Modrinth,
    Curseforge,
}

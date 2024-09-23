use clap::builder::styling::AnsiColor::{BrightBlue, White, Yellow};
use clap::{builder::Styles, Parser};

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
    subcommand: Subcommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Create and setup a new modpack.
    #[clap(visible_alias("new"))]
    Setup {
        /// The name of the newly created modpack.
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Manage modpack's components.
    Component {
        #[command(subcommand)]
        action: ComponentAction,
    },

    /// Export the modpack in one of the supported formats.
    #[clap(visible_alias("export"))]
    Forge,
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

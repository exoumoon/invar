mod cli;

use crate::cli::{ComponentAction, Options, PackAction, Subcommand};
use clap::Parser;
use inquire::validator::{StringValidator, Validation};
use invar::instance::{Instance, Loader};
use invar::local_storage::PersistedEntity;
use invar::{index::Index, pack::Pack};
use semver::Version;
use strum::IntoEnumIterator;
use tracing::{info, Level};

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented, so entering a non-existent version will result in an unusable modpack.";

#[expect(clippy::too_many_lines)]
fn main() {
    let _ = color_eyre::install();
    let options = Options::parse();

    tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .without_time()
        .compact()
        .init();

    let span = tracing::span!(Level::DEBUG, "invar");
    let _guard = span.enter();

    match options.subcommand {
        Subcommand::Pack { action } => match action {
            PackAction::Setup {
                mut name,
                mut minecraft_version,
                mut loader,
                mut loader_version,
                overwrite,
            } => {
                if !overwrite && Pack::read().is_ok() {
                    let confirmed = inquire::Confirm::new(
                    "A valid pack already exists in this directory, are you sure you wish to overwrite it with a new one?",
                )
                .with_placeholder("yes/no")
                .prompt()
                .unwrap_or(false);

                    if !confirmed {
                        std::process::exit(0);
                    }
                }

                let name = name.take().unwrap_or_else(|| {
                    inquire::Text::new("Modpack name:")
                        .with_validator(non_empty_validator("Please enter a non-empty name"))
                        .prompt()
                        .unwrap()
                        .trim()
                        .to_string()
                });

                let minecraft_version = minecraft_version.take().unwrap_or_else(|| {
                    inquire::CustomType::new("Minecraft version:")
                        .with_placeholder("X.X.X")
                        .with_help_message(VERSION_WARNING)
                        .with_error_message("That's not a valid semantic version.")
                        .prompt()
                        .unwrap()
                });

                let loader = loader.take().unwrap_or_else(|| {
                    inquire::Select::new("Modloader:", Loader::iter().collect::<Vec<_>>())
                        .prompt()
                        .unwrap()
                });

                let loader_version = match loader {
                    Loader::Minecraft => minecraft_version.clone(),
                    _ => loader_version.take().unwrap_or_else(|| {
                        inquire::CustomType::new("Modloader version:")
                            .with_placeholder("X.X.X")
                            .with_help_message(VERSION_WARNING)
                            .with_error_message("That's not a valid semantic version.")
                            .prompt()
                            .unwrap()
                    }),
                };

                let mut allowed_foreign_loaders = vec![];
                if loader == Loader::Neoforge {
                    // Neoforge should be compatible with Forge mods.
                    allowed_foreign_loaders.push(Loader::Forge);
                }
                if loader == Loader::Quilt {
                    // Quilt should be compatible with Fabric mods.
                    allowed_foreign_loaders.push(Loader::Fabric);
                }

                let pack = Pack {
                    name,
                    version: DEFAULT_PACK_VERSION,
                    authors: vec![], // TODO: Maybe add $USER by default?
                    instance: Instance {
                        minecraft_version,
                        loader,
                        loader_version,
                        allowed_foreign_loaders, // None by default.
                    },
                };

                pack.write().unwrap_or_else(|error| exit(1, error));
                Pack::setup_directories().unwrap_or_else(|error| exit(1, error));
                info!(
                    "Done. Check out `{pack_file}` for more options.",
                    pack_file = Pack::FILE_PATH
                );
            }

            PackAction::Show => {
                info!("Reading local storage...");
                let pack = Pack::read().unwrap_or_else(|error| exit(1, error));
                dbg!(&pack);
            }

            PackAction::Export => {
                info!("Reading local storage...");
                let pack = Pack::read().unwrap_or_else(|error| exit(1, error));
                let files = vec![]; // TODO
                let index = Index::from_pack_and_files(&pack, &files);
                info!("Exporting into `.mrpack` format...");
                let json = serde_json::to_string_pretty(&index).unwrap();
                println!("{json}");
            }
        },

        Subcommand::Component { action } => match action {
            ComponentAction::List => {
                let components = invar::component::load_components().unwrap();
                eprintln!("{components:#?}");
            }

            _ => todo!(),
        },
    }
}

fn non_empty_validator(error_msg: &str) -> impl StringValidator + '_ {
    |input: &str| match input.trim().is_empty() {
        true => Ok(Validation::Invalid(error_msg.into())),
        false => Ok(Validation::Valid),
    }
}

fn exit(code: i32, error: impl std::error::Error) -> ! {
    if code != 0 {
        tracing::error!("{error}");
        tracing::error!("Invar encountered an unrecoverable error and cannot continue.");
    }
    std::process::exit(code);
}

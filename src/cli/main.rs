use crate::cli::{ComponentAction, Options, PackAction, Subcommand};
use clap::Parser;
use color_eyre::eyre::Report;
use color_eyre::owo_colors::OwoColorize;
use eyre::Context;
use inquire::validator::{StringValidator, Validation};
use invar::component::Component;
use invar::index;
use invar::index::Index;
use invar::instance::{Instance, Loader};
use invar::local_storage::PersistedEntity;
use invar::pack::Pack;
use semver::Version;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use strum::IntoEnumIterator;
use tracing::{info, Level};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

mod cli;

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented, so entering a non-existent version will result in an unusable modpack.";

#[expect(clippy::too_many_lines)]
fn main() -> Result<(), Report> {
    color_eyre::install()?;
    let options = Options::parse();

    install_tracing();

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

                let mut allowed_foreign_loaders = vec![Loader::Minecraft];
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

                pack.write()?;
                Pack::setup_directories()?;
                info!(
                    "Done. Check out `{pack_file}` for more options.",
                    pack_file = Pack::FILE_PATH
                );
            }

            PackAction::Show => {
                info!("Reading local storage...");
                let pack = Pack::read()?;
                dbg!(&pack);
            }

            PackAction::Export => {
                let pack = Pack::read()?;
                let files: Vec<index::file::File> = invar::component::load_components()?
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let index = Index::from_pack_and_files(&pack, &files);
                let json = serde_json::to_string_pretty(&index)?;
                println!("{json}");
                let path = format!("{}.mrpack", pack.name);
                info!(message = "Writing index", target = ?path.yellow().bold());
                let file = File::create(path)?;
                let mut mrpack = ZipWriter::new(file);
                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                mrpack.start_file("modrinth.index.json", options)?;
                mrpack.write_all(json.as_bytes())?;
                mrpack.finish()?;
            }
        },

        Subcommand::Component { action } => match action {
            ComponentAction::List => {
                let components = invar::component::load_components()?;
                for c in &components {
                    println!(
                        "{type}: {tag}{slug} [{version}]",
                        type = c.category,
                        slug = c.slug.yellow().bold(),
                        version = c.file_name.bold(),
                        tag = match &c.tags.main {
                            Some(tag) => format!("{tag}/"),
                            None => String::new(),
                        }
                        .bright_yellow()
                        .bold(),
                    );
                }
                println!(
                    "{count} components in total.",
                    count = components.len().red().bold()
                );
            }

            ComponentAction::Add { ids, show_metadata } => {
                for id in ids {
                    let instance = Pack::read()
                        .wrap_err("Failed to read pack.yml. Is there a modpack in CWD?")?
                        .instance;
                    let component = Component::fetch_from_modrinth(&id, &instance).wrap_err(
                        format!("Failed to fetch the \"{id}\" component from Modrinth"),
                    )?;

                    info!(message = "Adding:", slug = ?id, file_name = ?component.file_name.yellow().bold());
                    if show_metadata {
                        let yaml = serde_yml::to_string(&component)
                            .wrap_err("Failed to serialize the component's metadata")?
                            .lines()
                            .fold(String::new(), |mut acc, line| {
                                let _ =
                                    writeln!(acc, "{prefix} {line}", prefix = "|>".yellow().bold());
                                acc
                            });
                        info!(message = "Writing metadata,", path = ?component.local_storage_path().yellow().bold());
                        print!("{yaml}");
                    }

                    component
                        .save_to_metadata_dir()
                        .wrap_err("Failed to save component's metadata")?;
                }
            }

            ComponentAction::Remove { slugs } => {
                for slug in &slugs {
                    let target_basename = format!("{slug}{}", Component::LOCAL_STORAGE_SUFFIX);
                    let candidate = invar::local_storage::metadata_files(".")
                        .wrap_err("Failed to load local metadata files")?
                        .find(|f| {
                            f.file_name()
                                .to_str()
                                .is_some_and(|name| name == target_basename)
                        });
                    match candidate {
                        Some(file) => {
                            info!("Removing {path:?}", path = file.path());
                            fs::remove_file(file.path()).wrap_err("Failed to remove file")?;
                        }
                        None => {
                            tracing::error!("Could not find a component with slug: {slug}");
                        }
                    }
                }
            }

            ComponentAction::Update { .. } => todo!(),
        },
    }

    Ok(())
}

fn non_empty_validator(error_msg: &str) -> impl StringValidator + '_ {
    |input: &str| match input.trim().is_empty() {
        true => Ok(Validation::Invalid(error_msg.into())),
        false => Ok(Validation::Valid),
    }
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

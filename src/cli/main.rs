use crate::cli::{ComponentAction, Options, PackAction, Subcommand};
use clap::Parser;
use cli::ServerAction;
use color_eyre::eyre::Report;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Section;
use eyre::Context;
use inquire::validator::{StringValidator, Validation};
use invar::local_storage::{Error, PersistedEntity};
use invar::server::docker_compose::DockerCompose;
use invar::server::Server;
use invar::{Component, Instance, Loader, Pack};
use semver::Version;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::{fs, io};
use strum::IntoEnumIterator;
use tracing::{info, instrument, Level};

mod cli;

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented, so entering a non-existent version may result in an unusable modpack.";

fn main() -> Result<(), Report> {
    let options = Options::parse();
    color_eyre::install()?;
    install_tracing()?;

    let span = tracing::span!(Level::DEBUG, "invar");
    let _guard = span.enter();

    let status = run_with_options(options);
    if let Err(mut report) = status {
        if let Some(error) = report.downcast_ref::<Error>() {
            match error {
                Error::Io { .. } => {
                    report = report
                        .with_note(|| "Invar encountered an I/O error.")
                        .with_suggestion(|| {
                            "Ensure you're in the right directory and have enough permissions."
                        });
                }
                Error::SerdeYml(_) | Error::SerdeJson(_) => {
                    report = report
                        .with_note(|| "Invar had an error while (de)serializing data with Serde.")
                        .with_note(|| "This really shouldn't happen, something is real broken.")
                        .with_suggestion(|| {
                            format!("Consider reporting this at {}", env!("CARGO_PKG_HOMEPAGE"))
                        });
                }
                Error::Walkdir(_) => {
                    report = report
                        .with_note(|| "Invar had an error while scanning modpack's files.")
                        .with_note(|| "Most likely there isn't a modpack in this directory.")
                        .with_suggestion(|| {
                            "Ensure you're in the right directory and have enough permissions."
                        });
                }
                Error::Zip(_) => {
                    report = report
                        .with_note(|| "Invar had an error while dealing with Zip archives.")
                        .with_note(|| "This really shouldn't happen, something is real broken.")
                        .with_suggestion(|| {
                            format!("Consider reporting this at {}", env!("CARGO_PKG_HOMEPAGE"))
                        });
                }
            }
        }

        return Err(report);
    }

    Ok(())
}

fn run_with_options(options: Options) -> Result<(), Report> {
    match options.subcommand {
        Subcommand::Pack { action } => match action {
            PackAction::Show => {
                println!("{}", serde_yml::to_string(&Pack::read()?)?);
                Ok(())
            }
            PackAction::Export => Ok(Pack::read()?.export()?),
            PackAction::Setup {
                name,
                minecraft_version,
                loader,
                loader_version,
                overwrite,
            } => setup_pack(name, minecraft_version, loader, loader_version, overwrite),
        },

        Subcommand::Component { action } => match action {
            ComponentAction::List => list_components(),
            ComponentAction::Add { ids, show_metadata } => add_component(&ids, show_metadata),
            ComponentAction::Remove { slugs } => remove_component(&slugs),
            ComponentAction::Update { .. } => {
                let error = eyre::eyre!("Updating components isn't yet implemented")
                    .with_note(|| "This will be implemented in a future version of Invar.")
                    .with_suggestion(|| "Remove and re-add this component to update it.");
                Err(error)
            }
        },

        Subcommand::Server { action, .. } => match action {
            ServerAction::Setup => DockerCompose::setup()
                .map(|_| ())
                .wrap_err("Failed to setup the server"),
            ServerAction::Start => DockerCompose::read()?
                .start()
                .wrap_err("Failed to start the server"),
            ServerAction::Stop => DockerCompose::read()?
                .stop()
                .wrap_err("Failed to stop the server"),
            ServerAction::Status => {
                let error = eyre::eyre!("Checking the status of the server isn't yet implemented")
                    .with_note(|| "This will be implemented in a future version of Invar.")
                    .with_suggestion(|| "`docker compose ps` may have what you need.");
                Err(error)
            }
            ServerAction::Backup { .. } => {
                let error = eyre::eyre!("Backups aren't yet implemented")
                    .with_note(|| "This will be implemented in a future version of Invar.");
                Err(error)
            }
        },
    }
}

#[instrument(level = "debug", ret)]
fn setup_pack(
    mut name: Option<String>,
    mut minecraft_version: Option<Version>,
    mut loader: Option<Loader>,
    mut loader_version: Option<Version>,
    overwrite: bool,
) -> Result<(), Report> {
    if !overwrite && fs::exists(<Pack as PersistedEntity>::FILE_PATH).is_ok_and(|exists| exists) {
        let confirmed = inquire::Confirm::new(
            "A pack already exists in this directory, are you sure you wish to overwrite it with a new one?",
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
    let mut allowed_foreign_loaders = HashSet::from_iter([Loader::Minecraft]);
    if loader == Loader::Forge || loader == Loader::Neoforge {
        // Neoforge should be compatible with Forge mods.
        allowed_foreign_loaders.extend([Loader::Forge, Loader::Neoforge]);
        allowed_foreign_loaders.remove(&loader);
    }
    if loader == Loader::Quilt {
        // Quilt should be compatible with Fabric mods.
        allowed_foreign_loaders.insert(Loader::Fabric);
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
    Ok(())
}

#[instrument(level = "debug", ret)]
fn remove_component(slugs: &[String]) -> Result<(), Report> {
    for slug in slugs {
        Component::remove(slug).wrap_err(format!("Failed to remove the {slug:?} component"))?;
    }

    Ok(())
}

#[instrument(level = "debug", ret)]
fn add_component(ids: &[String], show_metadata: bool) -> Result<(), Report> {
    let instance = Pack::read()?.instance;
    for id in ids {
        let component = Component::fetch_from_modrinth(id, &instance).wrap_err(format!(
            "Failed to fetch the {id:?} component from Modrinth"
        ))?;

        info!(message = "Adding:", slug = ?id, file_name = ?component.file_name.yellow().bold());
        if show_metadata {
            let yaml = serde_yml::to_string(&component)
                .wrap_err("Failed to serialize the component's metadata")?
                .lines()
                .fold(String::new(), |mut acc, line| {
                    let _ = writeln!(acc, "{prefix} {line}", prefix = "|>".yellow().bold());
                    acc
                });
            info!(message = "Writing metadata,", path = ?component.local_storage_path().yellow().bold());
            print!("{yaml}");
        }

        component
            .save_to_metadata_dir()
            .wrap_err("Failed to save component's metadata")?;
    }

    Ok(())
}

#[instrument(level = "debug", ret)]
fn list_components() -> Result<(), Report> {
    let components = invar::Component::load_all()?;
    for c in &components {
        println!(
            "{type}: {prefix}{slug} [{version}]",
            type = c.category,
            slug = c.slug.yellow().bold(),
            version = c.file_name.bold(),
            prefix = match &c.tags.main {
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
    Ok(())
}

fn install_tracing() -> Result<(), Report> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};
    let format_layer = fmt::layer().pretty().without_time().with_writer(io::stderr);
    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .with(ErrorLayer::default())
        .try_init()?;
    Ok(())
}

fn non_empty_validator(error_msg: &str) -> impl StringValidator + '_ {
    |input: &str| match input.trim().is_empty() {
        true => Ok(Validation::Invalid(error_msg.into())),
        false => Ok(Validation::Valid),
    }
}

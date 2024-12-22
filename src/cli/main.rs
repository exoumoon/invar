use crate::cli::{ComponentAction, Options, PackAction, Subcommand};
use clap::Parser;
use cli::ServerAction;
use color_eyre::eyre::Report;
use color_eyre::owo_colors::OwoColorize;
use docker_compose_types::{AdvancedVolumes, Compose, Service, Services, Volumes};
use eyre::Context;
use inquire::validator::{StringValidator, Validation};
use invar::component::Component;
use invar::index::Index;
use invar::instance::{Instance, Loader};
use invar::local_storage::PersistedEntity;
use invar::pack::Pack;
use invar::server::{DockerCompose, DEFAULT_MINECRAFT_PORT};
use invar::{index, local_storage};
use semver::Version;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::prelude::*;
use std::{fs, io};
use strum::IntoEnumIterator;
use tracing::{error, info, instrument, Level};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

mod cli;

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented, so entering a non-existent version may result in an unusable modpack.";

fn main() -> Result<(), Report> {
    color_eyre::install()?;
    let options = Options::parse();

    install_tracing();

    let span = tracing::span!(Level::DEBUG, "invar");
    let _guard = span.enter();

    let status = match options.subcommand {
        Subcommand::Pack { action } => match action {
            PackAction::Show => show_pack(),
            PackAction::Export => export_pack(),
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
                eyre::bail!("Updating components isn't yet implemented")
            }
        },

        Subcommand::Server { action, .. } => match action {
            ServerAction::Setup { force } => setup_server(force),
            _ => todo!(),
        },
    };

    match status {
        Ok(()) => Ok(()),
        Err(report) => {
            if let Some(invar_error) = report.downcast_ref::<local_storage::Error>() {
                match invar_error {
                    local_storage::Error::Io { .. } => {
                        error!("Looks like Invar encountered an I/O error. Most likely there isn't a modpack in this directory, or for some reason Invar cannot access files inside of it.");
                    }

                    local_storage::Error::Serde { .. } => {
                        error!("Looks like Invar had an error while (de)serializing data with Serde. This really isn't supposed to happen, something has to be real broken");
                    }

                    local_storage::Error::Walkdir { .. } => {
                        error!("Looks like Invar had an error while scanning modpack's files. Most likely there isn't a modpack in this directory, or for some reason Invar cannot access files inside of it.");
                    }
                }
            }

            error!("NOTE: Inspect the logs above and the the error chain below. Those should explain what happened.");

            Err(report)
        }
    }
}

#[instrument(level = "debug", ret)]
fn setup_server(force: bool) -> Result<(), Report> {
    let pack = Pack::read()?;

    let data_volume_path = "./server";
    if let Err(error) = fs::create_dir_all(data_volume_path) {
        match error.kind() {
            io::ErrorKind::AlreadyExists => {}
            _ => return Err(error).wrap_err("Failed to create a directory for the data volume"),
        }
    }

    let volumes = vec![
        // Minecraft's data (all kinds of state).
        Volumes::Advanced(AdvancedVolumes {
            source: Some(data_volume_path.into()),
            target: "/data".into(),
            _type: "bind".into(),
            read_only: false,
            bind: None,
            volume: None,
            tmpfs: None,
        }),
        // A "symlink" to our expored modpack.
        Volumes::Advanced(AdvancedVolumes {
            source: Some({
                export_pack()?;
                format!("./{}.mrpack", pack.name)
            }),
            target: DockerCompose::MODPACK_PATH.into(),
            _type: "bind".into(),
            read_only: true,
            bind: None,
            volume: None,
            tmpfs: None,
        }),
    ];

    let ports = docker_compose_types::Ports::Short(vec![format!(
        "{DEFAULT_MINECRAFT_PORT}:{DEFAULT_MINECRAFT_PORT}"
    )]);

    let mut services = HashMap::new();
    let environment = DockerCompose::build_environment(&pack.instance);
    let hostname = format!("{}_server", pack.name);
    let image = "itzg/minecraft-server:java17-alpine".to_string();
    services.insert(
        "server".to_string(),
        Some(Service {
            image: Some(image),
            hostname: Some(hostname.clone()),
            container_name: Some(hostname),
            environment,
            restart: Some("unless-stopped".into()),
            volumes,
            networks: docker_compose_types::Networks::Simple(vec![]),
            ports,
            ..Default::default()
        }),
    );

    let manifest = Compose {
        version: None,
        services: Services(services),
        volumes: docker_compose_types::TopLevelVolumes::default(),
        networks: docker_compose_types::ComposeNetworks::default(),
        service: None,
        secrets: None,
        extensions: HashMap::default(),
    };

    let manifest_path = <DockerCompose as PersistedEntity>::FILE_PATH;
    let manifest_exists = std::fs::exists(manifest_path)
        .wrap_err("Failed to check for an existing docker compose manifest")?;
    if manifest_exists && !force {
        let error = eyre::eyre!("A docker compose server manifest is already present");
        return Err(error.wrap_err("Pass `--force` if you wish to overwrite"));
    }

    let docker_compose = DockerCompose(manifest);
    docker_compose
        .write()
        .wrap_err("Failed ot save the server manifest to local storage")?;

    Ok(())
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
fn show_pack() -> Result<(), Report> {
    let pack = Pack::read()?;
    println!("{}", serde_yml::to_string(&pack)?);
    Ok(())
}

#[instrument(level = "debug", ret)]
fn export_pack() -> Result<(), Report> {
    let pack = Pack::read()?;
    let files: Vec<index::file::File> = invar::component::load_components()?
        .into_iter()
        .map(Into::into)
        .collect();
    let index = Index::from_pack_and_files(&pack, &files);
    let json = serde_json::to_string_pretty(&index)?;
    let path = format!("{}.mrpack", pack.name);
    info!(message = "Writing index", target = ?path.yellow().bold());
    let file = File::create(path)?;
    let mut mrpack = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    mrpack.start_file("modrinth.index.json", options)?;
    mrpack.write_all(json.as_bytes())?;
    mrpack.finish()?;
    Ok(())
}

#[instrument(level = "debug", ret)]
fn remove_component(slugs: &[String]) -> Result<(), Report> {
    for slug in slugs {
        let target_basename = format!("{slug}{}", Component::LOCAL_STORAGE_SUFFIX);
        let candidate = local_storage::metadata_files(".")
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

    Ok(())
}

#[instrument(level = "debug", ret)]
fn add_component(ids: &[String], show_metadata: bool) -> Result<(), Report> {
    for id in ids {
        let instance = Pack::read()?.instance;
        let component = Component::fetch_from_modrinth(id, &instance).wrap_err(format!(
            "Failed to fetch the \"{id}\" component from Modrinth"
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

    let format_layer = fmt::layer().pretty().without_time().with_writer(io::stderr);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .with(ErrorLayer::default())
        .init();
}

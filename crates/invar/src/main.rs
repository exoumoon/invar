mod cli;

use std::cell::LazyCell;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::{CommandFactory, Parser};
use cli::{BackupAction, ServerAction};
use color_eyre::eyre::Report;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Section;
use eyre::{Context, ContextCompat};
use inquire::validator::{StringValidator, Validation};
use invar_component::{
    Category, Component, Env, Id, LocalComponent, RemoteComponent, Requirement, RuntimeDirectory,
    Source, TagInformation,
};
use invar_pack::instance::version::MinecraftVersion;
use invar_pack::instance::{Instance, Loader};
use invar_pack::settings::Settings;
use invar_pack::Pack;
use invar_repository::persist::PersistedEntity;
use invar_repository::{LocalRepository, ModrinthRepository};
use invar_server::docker_compose::DockerCompose;
use invar_server::{backup, Server};
use itertools::Itertools;
use semver::Version;
use strum::IntoEnumIterator;
use tracing::instrument;

use crate::cli::{ComponentAction, Options, PackAction, Subcommand};

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented. Entering a non-existent version may result in an unusable modpack.";

fn main() -> Result<(), Report> {
    install_tracing_layer()?;

    let options = Options::parse();
    run(options)
}

#[instrument]
fn run(options: Options) -> Result<(), Report> {
    let modrinth_repository = LazyCell::new(ModrinthRepository::new);

    match options.subcommand {
        Subcommand::Pack { action } => match action {
            PackAction::Show => {
                let local_repository = LocalRepository::open_at_git_root()?;
                println!("{}", serde_yml::to_string(&local_repository.pack)?);
                Ok(())
            }
            PackAction::Export => {
                let local_repository = LocalRepository::open_at_git_root()?;
                let components = local_repository.components()?;
                local_repository.pack.export(components)?;
                Ok(())
            }
            PackAction::SetupDirectories => {
                let local_repository = LocalRepository::open_at_git_root()?;
                local_repository.setup_directories()?;
                Ok(())
            }
            PackAction::Setup {
                name,
                minecraft_version,
                loader,
                loader_version,
                overwrite,
            } => setup_pack(name, minecraft_version, loader, loader_version, overwrite),
        },

        Subcommand::Component { action } => match action {
            ComponentAction::List => {
                let local_repository = LocalRepository::open_at_git_root()?;
                for ref component @ Component {
                    ref id,
                    ref category,
                    ref environment,
                    ref source,
                    tags: _,
                } in local_repository
                    .components()?
                    .into_iter()
                    .sorted_by_key(|component| (component.category, component.id.clone()))
                {
                    match category {
                        Category::Mod => eprint!("{:>24}", id.bold().green()),
                        Category::Resourcepack => eprint!("{:>24}", id.bold().purple()),
                        Category::Shader => eprint!("{:>24}", id.bold().cyan()),
                        Category::Datapack => eprint!("{:>24}", id.bold().red()),
                        Category::Config => eprint!("{:>24}", id.bold().yellow()),
                    }

                    eprint!(
                        " [{source} {category}] ({environment}) runtime_path: {runtime_path:?} ",
                        source = source.cyan(),
                        category = category.blue().bold(),
                        environment = environment.purple().italic(),
                        runtime_path = PathBuf::from(component.runtime_path())
                            .display()
                            .bright_black(),
                    );

                    if let Source::Local(LocalComponent { path }) = source {
                        eprint!("source_file: {}", path.display().bright_black());
                    }

                    eprintln!(/* line termination */);
                }
                Ok(())
            }

            ComponentAction::Add {
                ids,
                local,
                forced_category,
            } => {
                let mut local_repository = LocalRepository::open_at_git_root()?;

                for id in &ids {
                    let source = if local {
                        let path = PathBuf::from(id).canonicalize()?;
                        let local_component = LocalComponent { path };
                        Source::Local(local_component)
                    } else {
                        let fetched_versions = modrinth_repository.fetch_versions(id)?;
                        let versions = fetched_versions
                            .into_iter()
                            .filter(|version| {
                                let instance = &local_repository.pack.instance;
                                let is_for_correct_version = version
                                    .game_versions
                                    .contains(&instance.minecraft_version.to_string());
                                let version_loaders: HashSet<Loader> =
                                    version.loaders.iter().copied().collect();
                                let has_supported_loader = instance
                                    .allowed_loaders()
                                    .intersection(&version_loaders)
                                    .count()
                                    >= 1;
                                is_for_correct_version && has_supported_loader
                            })
                            .sorted_unstable_by_key(|version| version.date_published)
                            .rev()
                            .collect::<Vec<_>>();

                        let help_msg = "Only ones with a matching MC version and loader are listed";
                        let prompt =
                            format!("Which version of {} should be added?", id.underline());
                        let selected_version = inquire::Select::new(&prompt, versions)
                            .with_help_message(help_msg)
                            .prompt()
                            .wrap_err("Failed to prompt for a component version")?;

                        let first_file = selected_version.files.into_iter().next().unwrap();
                        let remote_component = RemoteComponent {
                            download_url: first_file.url,
                            file_name: PathBuf::from(first_file.name),
                            file_size: first_file.size,
                            version_id: selected_version.id,
                            hashes: first_file.hashes,
                        };

                        Source::Remote(remote_component)
                    };

                    let category = match forced_category {
                        Some(forced_category) => forced_category,
                        None => match &source {
                            Source::Remote(_) => Category::Mod,
                            Source::Local(local_component) => {
                                let runtime_dir = local_component
                                    .path
                                    .parent()
                                    .and_then(Path::file_name)
                                    .and_then(OsStr::to_str)
                                    .wrap_err("Failed to figure out the component's parent dir")?
                                    .parse::<RuntimeDirectory>()
                                    .wrap_err("Failed to auto-categorize the component")?;
                                Category::from(runtime_dir)
                            }
                        },
                    };

                    let component = Component {
                        id: Id::from(id.clone()),
                        category,
                        tags: TagInformation::default(),
                        environment: Env {
                            client: Requirement::Required,
                            server: Requirement::Required,
                        },
                        source,
                    };

                    tracing::info!(?component.category, "Ready to save the component");

                    local_repository.save_component(&component)?;
                }

                Ok(())
            }

            ComponentAction::Remove { ids } => {
                let mut local_repository = LocalRepository::open_at_git_root()?;
                for id in ids {
                    local_repository
                        .remove_components(id)
                        .wrap_err("Failed to remove component")?;
                }
                Ok(())
            }

            ComponentAction::Update { .. } => {
                let error = eyre::eyre!("Updating components isn't yet implemented")
                    .with_note(|| "This will be implemented in a future version of Invar.")
                    .with_suggestion(|| "Remove and re-add this component to update it.");
                Err(error)
            }
        },

        Subcommand::Server { ref action, .. } => {
            let local_repository = LocalRepository::open_at_git_root()?;
            match action {
                ServerAction::Setup => DockerCompose::setup()
                    .map(|_| ())
                    .wrap_err("Failed to setup the server"),
                ServerAction::Start => DockerCompose::read()?
                    .start(&local_repository.pack)
                    .wrap_err("Failed to start the server"),
                ServerAction::Stop => DockerCompose::read()?
                    .stop(&local_repository.pack)
                    .wrap_err("Failed to stop the server"),
                ServerAction::Status => {
                    let error =
                        eyre::eyre!("Checking the status of the server isn't yet implemented")
                            .with_note(|| "This will be implemented in a future version of Invar.")
                            .with_suggestion(|| "`docker compose ps` may have what you need.");
                    Err(error)
                }

                ServerAction::Backup { action } => match action {
                    BackupAction::List => backup_list(&options),
                    BackupAction::Create => backup_create(&local_repository.pack),
                    BackupAction::Gc => backup_gc(&options),
                },
            }
        }

        Subcommand::Completions { shell } => {
            let mut command = Options::command();
            let bin_name = env!("CARGO_CRATE_NAME");
            let mut stdout = std::io::stdout();
            clap_complete::generate(shell, &mut command, bin_name, &mut stdout);
            Ok(())
        }
    }
}

#[instrument]
fn backup_list(options: &Options) -> Result<(), Report> {
    for backup in backup::get_all_backups()?.into_iter().rev() {
        println!("{backup}");
    }
    Ok(())
}

#[instrument]
fn backup_create(pack: &Pack) -> Result<(), Report> {
    backup::create_new(Some("ondemand"), pack)?;
    Ok(())
}

#[instrument]
fn backup_gc(options: &Options) -> Result<(), Report> {
    let gc_result = backup::gc().wrap_err("Failed to garbage-collect backups")?;
    if gc_result.removed.is_empty() {
        println!("All backups are fresh enough to keep.");
    } else {
        println!("Deleted the following backups:");
        for deleted_backup in gc_result.removed.iter().rev() {
            println!("{deleted_backup}");
        }
    }
    println!("Remaining backups:");
    for backup in gc_result.remaining.iter().rev() {
        println!("{backup}");
    }
    Ok(())
}

fn setup_pack(
    name: Option<String>,
    minecraft_version: Option<Version>,
    loader: Option<Loader>,
    loader_version: Option<Version>,
    overwrite: bool,
) -> Result<(), Report> {
    if !overwrite && fs::exists(<Pack as PersistedEntity>::FILE_PATH).is_ok_and(|exists| exists) {
        let confirmed = inquire::Confirm::new(
            "A pack already exists in this directory, are you sure you wish to overwrite it with a new one?",
        )
        .with_placeholder("yes")
        .prompt()
        .unwrap_or(false);

        if !confirmed {
            std::process::exit(0);
        }
    }
    let name = name.unwrap_or_else(|| {
        inquire::Text::new("Modpack name:")
            .with_validator(non_empty_validator("Please enter a non-empty name"))
            .prompt()
            .unwrap()
            .trim()
            .to_string()
    });
    let minecraft_version = minecraft_version.unwrap_or_else(|| {
        inquire::CustomType::new("Minecraft version:")
            .with_placeholder("X.X.X")
            .with_help_message(VERSION_WARNING)
            .with_error_message("That's not a valid semantic version.")
            .prompt()
            .unwrap()
    });
    let loader = loader.unwrap_or_else(|| {
        inquire::Select::new("Modloader:", Loader::iter().collect::<Vec<_>>())
            .prompt()
            .unwrap()
    });
    let loader_version = match loader {
        Loader::Minecraft => minecraft_version.clone(),
        _ => loader_version.unwrap_or_else(|| {
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
        instance: Instance {
            minecraft_version: MinecraftVersion::Semantic(minecraft_version),
            loader,
            loader_version,
            allowed_foreign_loaders, // None by default.
        },
        settings: Settings::default(),
        local_components: vec![],
    };

    pack.write()?;

    let local_repo = LocalRepository::open_at_git_root()?;
    local_repo.setup_directories()?;

    Ok(())
}

// #[instrument(level = "debug", ret)]
// fn list_components() -> Result<(), Report> {
//     for c in &components {
//         println!(
//             "{type}: {prefix}{slug} [{version}]",
//             type = c.category,
//             slug = c.slug.yellow().bold(),
//             version = c.file_name.bold(),
//             prefix = match &c.tags.main {
//                 Some(tag) => format!("{tag}/"),
//                 None => String::new(),
//             }
//             .bright_yellow()
//             .bold(),
//         );
//     }
//     println!(
//         "{count} components in total.",
//         count = components.len().red().bold()
//     );
//     Ok(())
// }

fn install_tracing_layer() -> Result<(), Report> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    color_eyre::install()?;

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

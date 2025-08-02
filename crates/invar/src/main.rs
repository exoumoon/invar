#![feature(result_option_map_or_default)]

mod cli;

use std::cell::LazyCell;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::{CommandFactory, Parser};
use cli::ServerAction;
use color_eyre::eyre::Report;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::{Section, eyre};
use eyre::{Context, ContextCompat};
use inquire::validator::{StringValidator, Validation};
use invar_component::{
    Category, Component, Env, Id, LocalComponent, RemoteComponent, RuntimeDirectory, Source,
    TagInformation,
};
use invar_pack::Pack;
use invar_pack::instance::version::MinecraftVersion;
use invar_pack::instance::{Instance, Loader};
use invar_pack::settings::Settings;
use invar_repository::models::Environment;
use invar_repository::persist::PersistedEntity;
use invar_repository::{LocalRepository, ModrinthRepository};
use invar_server::Server;
use invar_server::docker_compose::DockerCompose;
use itertools::Itertools;
use semver::Version;
use spinach::Spinner;
use strum::IntoEnumIterator;
use tracing::instrument;

use crate::cli::{ComponentAction, Options, PackAction, RepoAction, Subcommand};

const DEFAULT_PACK_VERSION: Version = Version::new(0, 1, 0);
const VERSION_WARNING: &str = "Version verification is not implemented. Entering a non-existent version may result in an unusable modpack.";

fn main() -> Result<(), Report> {
    install_tracing_layer()?;

    let options = Options::parse();
    run(options)
}

#[expect(clippy::too_many_lines)]
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
                let modpack_file_path = local_repository.modpack_file_path()?;
                local_repository
                    .pack
                    .export(components, &modpack_file_path)?;
                #[cfg(unix)]
                {
                    let link_path = format!(
                        "{export_directory}/{pack_name}-latest.mrpack",
                        export_directory = LocalRepository::EXPORT_DIRECTORY,
                        pack_name = local_repository.pack.name,
                    );
                    let _ = std::fs::remove_file(&link_path);
                    std::os::unix::fs::symlink(modpack_file_path.canonicalize()?, link_path)?;
                }
                Ok(())
            }
            PackAction::SetupDirectories => {
                let local_repository = LocalRepository::open_at_git_root()?;
                local_repository.setup()?;
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
                let components = local_repository.components()?;
                for component @ Component {
                    id,
                    category,
                    environment,
                    source,
                    tags: _,
                } in components
                    .iter()
                    .sorted_by_key(|component| (component.category, component.id.as_str()))
                {
                    match category {
                        Category::Mod => eprint!("{:>32}", id.bold().green()),
                        Category::Resourcepack => eprint!("{:>32}", id.bold().purple()),
                        Category::Shader => eprint!("{:>32}", id.bold().cyan()),
                        Category::Datapack => eprint!("{:>32}", id.bold().red()),
                        Category::Config => eprint!("{:>32}", id.bold().yellow()),
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

                    eprintln!(/* line termination */);
                }

                let total = components.len();
                let remote = components
                    .iter()
                    .filter(|component| component.source.is_remote())
                    .count();
                let local = components
                    .iter()
                    .filter(|component| component.source.is_local())
                    .count();
                eprintln!(
                    "{total:>35} total components ({remote} remote, {local} local)",
                    total = total.bold(),
                    remote = remote.bold(),
                    local = local.bold(),
                );

                Ok(())
            }

            ComponentAction::Add {
                ids,
                local,
                forced_category,
            } => {
                let mut local_repository = LocalRepository::open_at_git_root()?;

                for id in ids {
                    if local {
                        let path = PathBuf::from(&id).canonicalize()?;
                        let parent_dir = path
                            .parent()
                            .and_then(Path::file_name)
                            .and_then(OsStr::to_str)
                            .wrap_err("Failed to figure out the component's parent dir")?
                            .parse::<RuntimeDirectory>()
                            .wrap_err("Failed to auto-categorize the component")?;
                        let component = Component {
                            id: Id::from(id),
                            source: Source::Local(LocalComponent { path }),
                            environment: Env::client_and_server(),
                            tags: TagInformation::untagged(), // TODO: Figure out tags.
                            category: forced_category.unwrap_or_else(|| Category::from(parent_dir)),
                        };
                        local_repository.save_component(&component)?;
                    } else {
                        add_component_from_modrinth(
                            &mut local_repository,
                            &modrinth_repository,
                            id,
                            forced_category,
                        )?;
                    }
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

        Subcommand::Server { ref action, .. } => match action {
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
        },

        Subcommand::Repo { action } => match action {
            RepoAction::Show => {
                let repo = LocalRepository::open_at_git_root()?;
                eprintln!("root_directory: {}", repo.root_directory.display());
                eprintln!("pack:\n{:#?}", repo.pack);
                Ok(())
            }
        },

        Subcommand::Completions { shell } => {
            let mut command = Options::command();
            let bin_name = env!("CARGO_CRATE_NAME");
            let mut stdout = std::io::stdout();
            clap_complete::generate(shell, &mut command, bin_name, &mut stdout);
            Ok(())
        }
    }
}

#[expect(clippy::equatable_if_let, reason = "looks ugly")]
fn setup_pack(
    name: Option<String>,
    minecraft_version: Option<Version>,
    loader: Option<Loader>,
    loader_version: Option<Version>,
    overwrite: bool,
) -> Result<(), Report> {
    if !overwrite
        && let message = "A pack already exists in this directory, are you sure you wish to overwrite it with a new one?"
        && let Ok(true) = fs::exists(Pack::FILE_PATH)
        && let false = inquire::Confirm::new(message)
            .with_placeholder("yes/no")
            .prompt()
            .unwrap_or(false)
    {
        tracing::info!("Pack overwrite not confirmed, exiting");
        return Ok(());
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

    let local_repo = LocalRepository::open(".")?;
    local_repo.setup()?;

    Ok(())
}

#[expect(clippy::too_many_lines)]
fn add_component_from_modrinth<S>(
    local_repository: &mut LocalRepository,
    modrinth_repository: &ModrinthRepository,
    id: S,
    forced_category: Option<Category>,
) -> Result<(), Report>
where
    S: AsRef<str> + Clone + std::fmt::Debug + std::fmt::Display,
    Id: std::convert::From<S>,
{
    let installed_components = local_repository.components()?;

    if installed_components
        .iter()
        .any(|component| component.id == id.as_ref().into())
    {
        eprintln!("- {} is already installed", id.green().bold());
        return Ok(());
    }

    let spinner_text = &format!("Fetching {} versions from Modrinth", id.underline());
    let spinner = Spinner::new(spinner_text).start();
    let instance = &local_repository.pack.instance;
    let versions = modrinth_repository
        .fetch_versions(&id)?
        .into_iter()
        .filter(|version| version.is_compatible(instance))
        .sorted_unstable_by_key(|version| version.date_published)
        .rev()
        .collect::<Vec<_>>();
    spinner.text("Fetch complete").success();

    if versions.is_empty() {
        let loaders = instance.allowed_loaders();
        let note = format!("No version is compatible with any of: {loaders:?}");
        let suggestion = "If a cross-loader compatibility layer like Connector is present, remember to tweak the allowed foreign loaders";
        let report = eyre::eyre!("No compatible versions of {id:?} found")
            .with_note(|| note)
            .with_suggestion(|| suggestion);
        return Err(report);
    }

    let help_msg = "Only ones with a matching MC version and loader are listed";
    let prompt = format!("Which version of {} should be added?", id.underline());
    let mut selected_version = inquire::Select::new(&prompt, versions)
        .with_help_message(help_msg)
        .prompt()
        .wrap_err("Failed to prompt for a component version")?;

    let spinner = Spinner::new("Resolving dependency names").start();
    selected_version.dependencies.retain_mut(|dependency| {
        let text = &format!("Resolving project ID: {}", &dependency.project_id.purple());
        spinner.text(text).update();
        match modrinth_repository.fetch_project(&dependency.project_id) {
            Ok(project) => {
                dependency.project_id = project.slug;
                dependency.display_name = Some(project.name);
                dependency.summary = project.summary;
                true
            }
            Err(error) => {
                tracing::warn!(?error, dependency.project_id, "Dependency resolution error");
                false
            }
        }
    });
    spinner.text("All dependency names resolved").success();

    let mut pending_dependencies = vec![];
    pending_dependencies.extend(selected_version.required_dependencies().cloned());

    let optional_deps = selected_version
        .optional_dependencies()
        .sorted_unstable_by_key(|dependency| dependency.project_id.as_str())
        .cloned()
        .collect::<Vec<_>>();
    if !optional_deps.is_empty() {
        let message = format!("{} has optional dependencies:", id.purple());
        let selected = inquire::MultiSelect::new(&message, optional_deps).prompt()?;
        pending_dependencies.extend(selected);
    }

    for installed_dependency in pending_dependencies.extract_if(.., |dependency| {
        installed_components
            .iter()
            .any(|component| *component.id == dependency.project_id)
    }) {
        eprintln!(
            "- {id} ({type:?}): already installed",
            id = installed_dependency.project_id.green().bold(),
            type = installed_dependency.dependency_type.bold(),
        );
    }

    for pending_dependency in &pending_dependencies {
        eprintln!(
            "- {id} ({type:?}): pending",
            id = pending_dependency.project_id.red().bold(),
            type = pending_dependency.dependency_type.bold(),
        );
    }

    let first_file = selected_version.files.into_iter().next().unwrap();
    let remote_component = RemoteComponent {
        download_url: first_file.url,
        file_name: PathBuf::from(first_file.name),
        file_size: first_file.size,
        version_id: selected_version.id,
        hashes: first_file.hashes,
    };

    let category = forced_category.unwrap_or(
        selected_version
            .project_types
            .into_iter()
            .next()
            .wrap_err("Component has no project types")?,
    );

    let component = Component {
        id: Id::from(id),
        category,
        tags: TagInformation::untagged(),
        environment: selected_version
            .environment
            .unwrap_or(match category {
                Category::Resourcepack | Category::Shader => Environment::ClientOnly,
                Category::Mod | Category::Datapack | Category::Config => {
                    Environment::ClientAndServer
                }
            })
            .into(),
        source: Source::Remote(remote_component),
    };

    local_repository.save_component(&component)?;
    for pending_dependency in &pending_dependencies {
        add_component_from_modrinth::<&str>(
            local_repository,
            modrinth_repository,
            pending_dependency.project_id.as_str(),
            forced_category,
        )?;
    }

    Ok(())
}

fn install_tracing_layer() -> Result<(), Report> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, fmt};

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

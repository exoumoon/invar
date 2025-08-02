use std::collections::HashMap;
use std::{fs, io};

use bon::bon;
use docker_compose_types::{AdvancedVolumes, Compose, Environment, Service, SingleValue, Volumes};
use invar_pack::instance::Instance;
use invar_repository::LocalRepository;
use invar_repository::persist::PersistedEntity;
use serde::{Deserialize, Serialize};

use super::{DEFAULT_MINECRAFT_PORT, Difficulty, Gamemode, Server};

pub const DATA_VOLUME_PATH: &str = "server";
pub const DEFAULT_ICON_URL: &str = "https://avatars.githubusercontent.com/u/175053991?s=200&v=4";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DockerCompose(pub Compose);

impl PersistedEntity for DockerCompose {
    const FILE_PATH: &'static str = "docker-compose.yaml";
}

#[bon]
impl DockerCompose {
    pub const MODPACK_PATH: &'static str = "/data/modpack.mrpack";

    #[builder]
    #[must_use]
    pub fn environment(
        instance: &Instance,
        operator_username: &str,
        memlimit_gb: u8,
        max_players: u16,
        online_mode: bool,
        allow_flight: bool,
        gamemode: &Gamemode,
        difficulty: &Difficulty,
        motd: String,
    ) -> Environment {
        let kv_pairs = [
            ("EULA", SingleValue::String("TRUE".into())),
            (
                "VERSION",
                SingleValue::String(instance.minecraft_version.to_string()),
            ),
            ("TYPE", SingleValue::String("MODRINTH".into())),
            (
                format!("{}_VERSION", instance.loader.to_string().to_uppercase()).as_str(),
                SingleValue::String(instance.loader_version.to_string()),
            ),
            (
                "MODRINTH_MODPACK",
                SingleValue::String(Self::MODPACK_PATH.into()),
            ),
            ("MEMORY", SingleValue::String(format!("{memlimit_gb}G"))),
            ("USE_AIKAR_FLAGS", SingleValue::Bool(true)),
            ("ENABLE_AUTOPAUSE", SingleValue::Bool(false)),
            ("VIEW_DISTANCE", SingleValue::Unsigned(8)),
            ("MODE", SingleValue::String(gamemode.to_string())),
            ("DIFFICULTY", SingleValue::String(difficulty.to_string())),
            ("MAX_PLAYERS", SingleValue::Unsigned(max_players.into())),
            ("MOTD", SingleValue::String(motd)),
            ("ICON", SingleValue::String(DEFAULT_ICON_URL.into())),
            ("ALLOW_FLIGHT", SingleValue::Bool(allow_flight)),
            ("ONLINE_MODE", SingleValue::Bool(online_mode)),
            {
                let rcon_first_connect = indoc::indoc! {"
                        /whitelist off
                        /op username
                    "}
                .replace("username", operator_username);
                (
                    "RCON_CMDS_FIRST_CONNECT",
                    SingleValue::String(rcon_first_connect),
                )
            },
        ]
        .map(|(key, value)| (key.to_string(), Some(value)));
        Environment::KvPair(HashMap::from_iter(kv_pairs))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum SetupError {
    #[error("A local server is already configured for this pack")]
    AlreadySetUp,
    #[error("An I/O error occurred")]
    Io(#[from] io::Error),
    #[error("Failed to interact with the local repository")]
    Repository(#[from] invar_repository::Error),
    #[error("Failed to export the pack before setup")]
    ExportFailed(#[from] invar_pack::ExportError),
    Git(#[from] git2::Error),
}

impl Server for DockerCompose {
    type SetupError = self::SetupError;

    fn setup() -> Result<Self, Self::SetupError> {
        let local_repo = LocalRepository::open_at_git_root()?;

        if let Err(error) = fs::create_dir_all(DATA_VOLUME_PATH)
            && error.kind() != io::ErrorKind::AlreadyExists
        {
            return Err(error.into());
        }

        // HACK: The must be a valid `.mrpack` for the docker volume to point to.
        local_repo
            .pack
            .export(local_repo.components()?, &local_repo.modpack_file_path()?)?;

        let volumes = vec![
            Volumes::Advanced(AdvancedVolumes {
                source: Some(DATA_VOLUME_PATH.into()),
                target: "/data".into(),
                _type: "bind".into(),
                read_only: false,
                bind: None,
                volume: None,
                tmpfs: None,
            }),
            Volumes::Advanced(AdvancedVolumes {
                source: Some({
                    format!(
                        "{}/{}-latest.mrpack",
                        LocalRepository::EXPORT_DIRECTORY,
                        local_repo.pack.name
                    )
                }),
                target: Self::MODPACK_PATH.into(),
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

        let hostname = local_repo.pack.name.clone();
        let image = "itzg/minecraft-server:java21".to_string();
        let motd = format!(
            "{pkg_name}/{pkg_version} | {pack_name} | {mc_version}",
            pkg_name = env!("CARGO_PKG_NAME"),
            pkg_version = env!("CARGO_PKG_VERSION"),
            pack_name = local_repo.pack.name,
            mc_version = local_repo.pack.instance.minecraft_version,
        );

        let environment = Self::environment()
            .instance(&local_repo.pack.instance)
            .operator_username("mxxntype")
            .memlimit_gb(16)
            .max_players(8)
            .online_mode(false)
            .allow_flight(true)
            .gamemode(&Gamemode::Survival)
            .difficulty(&Difficulty::Hard)
            .motd(motd)
            .call();

        let services = HashMap::from([(
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
        )]);

        let manifest = Compose {
            includes: None,
            version: None,
            services: docker_compose_types::Services(services),
            volumes: docker_compose_types::TopLevelVolumes::default(),
            networks: docker_compose_types::ComposeNetworks::default(),
            service: None,
            secrets: None,
            extensions: HashMap::default(),
            name: None,
        };

        let manifest_path = Self::FILE_PATH;
        if std::fs::exists(manifest_path)? {
            tracing::warn!("Server is already set up. Delete {manifest_path:?} for re-setup",);
            return Err(SetupError::AlreadySetUp);
        }

        let docker_compose = Self(manifest);
        docker_compose
            .write()
            .map_err(invar_repository::Error::Persistence)?;

        Ok(docker_compose)
    }
}

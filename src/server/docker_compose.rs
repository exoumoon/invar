use super::{Difficulty, Gamemode, Server, DEFAULT_MINECRAFT_PORT};
use crate::instance::Instance;
use crate::local_storage;
use crate::local_storage::PersistedEntity;
use crate::pack::Pack;
use bon::bon;
use docker_compose_types::{AdvancedVolumes, Compose, Environment, Service, SingleValue, Volumes};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fs, io};

const DEFAULT_ICON_URL: &str =
    "https://raw.githubusercontent.com/exoumoon/ground-zero/main/assets/icon.png";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DockerCompose(pub Compose);

impl PersistedEntity for DockerCompose {
    const FILE_PATH: &'static str = "docker-compose.yml";
}

#[allow(clippy::empty_enum, reason = "Rises from within bon")]
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
            ("ENABLE_AUTOPAUSE", SingleValue::Bool(true)),
            ("VIEW_DISTANCE", SingleValue::Unsigned(12)),
            ("MODE", SingleValue::String(gamemode.to_string())),
            ("DIFFICULTY", SingleValue::String(difficulty.to_string())),
            ("MAX_PLAYERS", SingleValue::Unsigned(max_players.into())),
            ("MOTD", SingleValue::String("TODO".into())),
            ("ICON", SingleValue::String(DEFAULT_ICON_URL.into())),
            ("ALLOW_FLIGHT", SingleValue::Bool(allow_flight)),
            ("ONLINE_MODE", SingleValue::Bool(online_mode)),
            {
                let rcon_first_connect = indoc::indoc! {"
                        /whitelist on
                        /whitelist add username
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
        let kv_hashmap = HashMap::from_iter(kv_pairs);

        Environment::KvPair(kv_hashmap)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("A local server is already configured for this pack")]
    AlreadySetUp,
    #[error(transparent)]
    Other(#[from] local_storage::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StartStopError {
    #[error(transparent)]
    ExitCode(#[from] io::Error),
    #[error("Process terminated by signal")]
    Terminated,
}

impl Server for DockerCompose {
    type SetupError = self::SetupError;
    type StartStopError = self::StartStopError;

    fn setup() -> Result<Self, Self::SetupError> {
        let pack = Pack::read()?;

        let data_volume_path = "./server";
        if let Err(error) = fs::create_dir_all(data_volume_path) {
            match error.kind() {
                io::ErrorKind::AlreadyExists => {}
                _ => return Err(local_storage::Error::from(error).into()),
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
            // A "symlink" to our exported modpack.
            Volumes::Advanced(AdvancedVolumes {
                source: Some({
                    pack.export()?;
                    format!("./{}.mrpack", pack.name)
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

        let hostname = format!("{}_server", pack.name);
        let image = "itzg/minecraft-server:java17-alpine".to_string();
        let environment = Self::environment()
            .instance(&pack.instance)
            .operator_username("mxxntype")
            .memlimit_gb(12)
            .max_players(4)
            .online_mode(false)
            .allow_flight(true)
            .gamemode(&Gamemode::Survival)
            .difficulty(&Difficulty::Hard)
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
            version: None,
            services: docker_compose_types::Services(services),
            volumes: docker_compose_types::TopLevelVolumes::default(),
            networks: docker_compose_types::ComposeNetworks::default(),
            service: None,
            secrets: None,
            extensions: HashMap::default(),
        };

        let manifest_path = <Self as PersistedEntity>::FILE_PATH;
        match std::fs::exists(manifest_path) {
            Ok(true) => {
                tracing::warn!(
                    "A {server_type:?} server is already set up. Delete {manifest_path:?} before re-setup",
                    server_type = std::any::type_name::<Self>()
                );
                return Err(SetupError::AlreadySetUp);
            }
            Err(error) => return Err(local_storage::Error::from(error).into()),
            _ => { /* All fine, go on */ }
        }

        let docker_compose = Self(manifest);
        docker_compose.write()?;
        Ok(docker_compose)
    }

    fn start(&self) -> Result<(), Self::StartStopError> {
        let status = std::process::Command::new("docker")
            .args([
                "compose",
                "--file",
                <Self as PersistedEntity>::FILE_PATH,
                "up",
                "--detach",
            ])
            .status()?;
        if let Some(status_code) = status.code() {
            match status_code {
                0 => Ok(()),
                error => Err(io::Error::from_raw_os_error(error).into()),
            }
        } else {
            Err(StartStopError::Terminated)
        }
    }

    fn stop(&self) -> Result<(), Self::StartStopError> {
        let status = std::process::Command::new("docker")
            .args([
                "compose",
                "--file",
                <Self as PersistedEntity>::FILE_PATH,
                "down",
            ])
            .status()?;
        if let Some(status_code) = status.code() {
            match status_code {
                0 => Ok(()),
                error => Err(io::Error::from_raw_os_error(error).into()),
            }
        } else {
            Err(StartStopError::Terminated)
        }
    }
}

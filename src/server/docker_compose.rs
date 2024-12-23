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
        let environment_hashmap = {
            let mut environment = HashMap::new();
            environment.insert("EULA".into(), Some(SingleValue::String("TRUE".into())));
            environment.insert(
                "VERSION".into(),
                Some(SingleValue::String(instance.minecraft_version.to_string())),
            );
            environment.insert("TYPE".into(), Some(SingleValue::String("MODRINTH".into())));
            environment.insert(
                format!("{}_VERSION", instance.loader.to_string().to_uppercase()),
                Some(SingleValue::String(instance.loader_version.to_string())),
            );
            environment.insert(
                "MODRINTH_MODPACK".into(),
                Some(SingleValue::String(Self::MODPACK_PATH.into())),
            );

            // TODO: Figure out how much MEMORY to allocate.
            environment.insert(
                "MEMORY".into(),
                Some(SingleValue::String(format!("{memlimit_gb}G"))),
            );
            environment.insert("USE_AIKAR_FLAGS".into(), Some(SingleValue::Bool(true)));
            environment.insert("ENABLE_AUTOPAUSE".into(), Some(SingleValue::Bool(true)));
            environment.insert("VIEW_DISTANCE".into(), Some(SingleValue::Unsigned(12)));
            environment.insert(
                "MODE".into(),
                Some(SingleValue::String(gamemode.to_string())),
            );
            environment.insert(
                "DIFFICULTY".into(),
                Some(SingleValue::String(difficulty.to_string())),
            );
            environment.insert(
                "MAX_PLAYERS".into(),
                Some(SingleValue::Unsigned(max_players.into())),
            );
            environment.insert("MOTD".into(), Some(SingleValue::String("TODO".into())));
            environment.insert(
                "ICON".into(),
                Some(SingleValue::String(
                    // TODO: Inject an icon.
                    "https://raw.githubusercontent.com/exoumoon/ground-zero/main/assets/icon.png"
                        .into(),
                )),
            );
            environment.insert("ALLOW_FLIGHT".into(), Some(SingleValue::Bool(allow_flight)));
            environment.insert("ONLINE_MODE".into(), Some(SingleValue::Bool(online_mode)));

            // TODO: Inject the username.
            let rcon_first_connect = indoc::indoc! {"
                /whitelist on
                /whitelist add username
                /op username
            "}
            .replace("username", operator_username);
            environment.insert(
                "RCON_CMDS_FIRST_CONNECT".into(),
                Some(SingleValue::String(rcon_first_connect)),
            );
            environment
        };

        Environment::KvPair(environment_hashmap)
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
            // A "symlink" to our expored modpack.
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

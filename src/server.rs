use crate::instance::Instance;
use crate::local_storage;
use crate::local_storage::PersistedEntity;
use docker_compose_types::{Compose, Environment, SingleValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub const DEFAULT_MINECRAFT_PORT: u16 = 25565;

pub trait Server: fmt::Debug + Serialize + for<'de> Deserialize<'de> {
    type Error;
    type Status;

    /// Prepare everything for the first start of the server.
    ///
    /// # Errors
    ///
    /// ...
    fn setup(&self) -> Result<(), local_storage::Error>;

    /// Start the hosted server, do nothing if it is already running.
    ///
    /// # Errors
    ///
    /// ...
    fn start(&self) -> Result<(), Self::Error>;

    /// Stop the hosted server, do nothing if it is already stopped.
    ///
    /// # Errors
    ///
    /// ...
    fn stop(&self) -> Result<(), Self::Error>;

    /// Report the status of the server.
    ///
    /// # Errors
    ///
    /// ...
    fn status(&self) -> Result<Self::Status, Self::Error>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DockerCompose(pub Compose);

impl PersistedEntity for DockerCompose {
    const FILE_PATH: &'static str = "docker-compose.yml";
}

impl DockerCompose {
    pub const MODPACK_PATH: &'static str = "/data/modpack.mrpack";

    #[must_use]
    pub fn build_environment(instance: &Instance) -> Environment {
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
            environment.insert("MEMORY".into(), Some(SingleValue::String("12G".into())));
            environment.insert("USE_AIKAR_FLAGS".into(), Some(SingleValue::Bool(true)));
            environment.insert("ENABLE_AUTOPAUSE".into(), Some(SingleValue::Bool(true)));
            environment.insert("VIEW_DISTANCE".into(), Some(SingleValue::Unsigned(12)));
            environment.insert("MODE".into(), Some(SingleValue::String("survival".into())));
            environment.insert(
                "DIFFICULTY".into(),
                Some(SingleValue::String("hard".into())),
            );
            environment.insert("MAX_PLAYERS".into(), Some(SingleValue::Unsigned(4)));
            environment.insert("MOTD".into(), Some(SingleValue::String("TODO".into())));
            environment.insert(
                "ICON".into(),
                Some(SingleValue::String(
                    // TODO: Inject an icon.
                    "https://raw.githubusercontent.com/exoumoon/ground-zero/main/assets/icon.png"
                        .into(),
                )),
            );
            environment.insert("ALLOW_FLIGHT".into(), Some(SingleValue::Bool(true)));
            environment.insert("ONLINE_MODE".into(), Some(SingleValue::Bool(false)));

            // TODO: Inject the username.
            let operator_username = "mxxntype";
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

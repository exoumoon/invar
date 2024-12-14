use clap::ValueEnum;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumIter};

/// A struct representing a Minecraft instance.
///
/// An instance does NOT take into account the associated `Component`s. Those
/// bounded with an `Instance`, all the `Config`s and `Override`s form a `Pack`.
/// See `Pack` if that's what you're looking for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub minecraft_version: Version,
    pub loader: Loader,
    pub loader_version: Version,

    /// Mods with an incompatible loader will be allowed in the pack if this
    /// list contains their loader.
    ///
    /// Some mods like [Sinytra Connector](https://github.com/Sinytra/Connector) allow loading Fabric mods
    /// on Forge/NeoForge, this option makes it so you can add mods that would
    /// require a compatibility layer without getting bombarded with
    /// incompatibility warnings.
    pub allowed_foreign_loaders: Vec<Loader>,
}

impl Instance {
    #[must_use = "Unused instance dependencies"]
    pub fn index_dependencies(&self) -> HashMap<Loader, Version> {
        let mut dependencies = HashMap::new();
        dependencies.insert(self.loader.clone(), self.loader_version.clone());
        dependencies.insert(Loader::Minecraft, self.minecraft_version.clone());
        dependencies
    }
}

/// Possible types of modloaders an instance can depend on.
///
/// Implements [`serde`]'s (De)serialization and [`clap`]'s [`ValueEnum`].
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ValueEnum, EnumIter, Display, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    /// Vanilla minecraft with no external modloader.
    ///
    /// You won't be able to load mods (and thus shaders) if this is the
    /// [`Instance::loader`].
    #[serde(alias = "vanilla", alias = "none", alias = "datapack")]
    Minecraft,

    /// The [**Forge**](https://minecraftforge.net) modloader.
    ///
    /// Has been around for a long time, and still kinda is the goto loader.
    Forge,

    /// The [**NeoForge**](https://neoforged.net) modloader.
    ///
    /// It is compatible with [`Forge`](Loader::Forge), is open-source and
    /// fresh, but is only available for newer versions.
    Neoforge,

    /// The [**Fabric**](https://fabricmc.net) modloader.
    ///
    /// Fabric is a modular, lightweight mod loader for Minecraft. Performant
    /// and fresh, but incompatible with [`Forge`](Loader::Forge) mods.
    Fabric,

    /// The [**Quilt**](https://quiltmc.org/en) modloader.
    ///
    /// An open-source, community-driven modding toolchain. I believe its
    /// [`Fabric`](Loader::Fabric)-compatible in terms of mods.
    Quilt,

    /// Some other modloader we don't know about.
    ///
    /// Shaders sometimes say their loader is `"iris"` or `"optifine"`, mods may
    /// just say `"modloader"`. In these cases, it's up to the user to check
    /// that the component is compatible with the instance.
    #[serde(other)]
    Other,
}

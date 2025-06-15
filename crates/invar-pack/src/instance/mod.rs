use std::collections::{HashMap, HashSet};

use semver::Version;
use serde::{Deserialize, Serialize};
use version::MinecraftVersion;

/// Some domain-specific types representing Minecraft's version formats.
pub mod version;

/// A struct representing a **Minecraft instance**.
///
/// An instance does **NOT** take into account the associated [`Components`].
/// Those bounded with an [`Instance`], all the configuration files and
/// overrides form a [`Pack`]. See [`Pack`] if that's what you're looking for.
///
/// [`Components`]: invar_component::Component
/// [`Pack`]: crate::Pack
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[must_use]
pub struct Instance {
    pub minecraft_version: MinecraftVersion,
    pub loader: Loader,
    pub loader_version: Version,

    /// Mods with an incompatible loader will be allowed in the pack if this
    /// list contains their loader.
    ///
    /// Some mods like [Sinytra Connector](https://github.com/Sinytra/Connector)
    /// allow loading Fabric mods on Forge/NeoForge, this option makes it so you
    /// can add mods that would require a compatibility layer without getting
    /// bombarded with incompatibility warnings.
    pub allowed_foreign_loaders: HashSet<Loader>,
}

impl Instance {
    /// Creates a new [`Instance`].
    ///
    /// Also fits the resulting [`Instance`] with a predefined set of
    /// [`allowed_foreign_loaders`], based on the provided `loader`.
    ///
    /// [`allowed_foreign_loaders`]: Self::allowed_foreign_loaders
    pub fn new(
        minecraft_version: MinecraftVersion,
        loader: Loader,
        loader_version: Version,
    ) -> Self {
        let mut allowed_foreign_loaders = HashSet::new();
        if loader != Loader::Minecraft {
            allowed_foreign_loaders.extend([Loader::Minecraft]);
        }
        match loader {
            Loader::Forge => allowed_foreign_loaders.extend([Loader::Neoforge]),
            Loader::Neoforge => allowed_foreign_loaders.extend([Loader::Forge]),
            Loader::Fabric => allowed_foreign_loaders.extend([Loader::Quilt]),
            Loader::Quilt => allowed_foreign_loaders.extend([Loader::Fabric]),
            Loader::Other | Loader::Minecraft => { /* nothing */ }
        }

        Self {
            minecraft_version,
            loader,
            loader_version,
            allowed_foreign_loaders,
        }
    }

    #[must_use]
    pub fn index_dependencies(&self) -> HashMap<Loader, String> {
        HashMap::from_iter([
            (self.loader, self.loader_version.to_string()),
            (Loader::Minecraft, self.minecraft_version.to_string()),
        ])
    }
}

/// Possible types of modloaders an [`Instance`] can depend on.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
    strum::EnumIter,
    strum::Display,
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
    /// Modular, lightweight mod loader. Performant and fresh, but incompatible
    /// with [`Forge`](Loader::Forge) mods.
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

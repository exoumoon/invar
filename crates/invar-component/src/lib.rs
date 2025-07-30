//! This crate is a part of **[Invar]**.
//!
//! ## What's a component?
//!
//! Components are basically the main building blocks of a modpack - mods,
//! resourcepacks, shaderpacks, datapacks, configuration files and such are all
//! components. See the [`Category`] enum, that one is a list of things that are
//! considered components.
//!
//! This crate does not implement any manipulation upon components, it only
//! provides types to be used by other parts of **[Invar]**.
//!
//! [Invar]: https://github.com/exoumoon/invar

use std::fmt;
use std::path::PathBuf;

use clap::ValueEnum;
use nutype::nutype;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::Display;
use url::Url;

mod runtime;
mod tag;
pub use runtime::*;
pub use tag::*;

/// An identifier of a [`Component`].
#[nutype(
    sanitize(trim, lowercase),
    derive(
        From,
        Into,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Display,
        Clone,
        Debug,
    )
)]
pub struct Id(String);

/// A **runtime modpack component** - a mod, shaderpack, etc.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[must_use]
pub struct Component {
    pub id: Id,
    pub category: Category,
    pub tags: TagInformation,
    pub environment: Env,
    pub source: Source,
}

/// Possible sources where a [`Component`] might come from.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[must_use]
#[expect(clippy::large_enum_variant)]
pub enum Source {
    Remote(RemoteComponent),
    Local(LocalComponent),
}

impl Source {
    /// Returns the file name of this [`Source`].
    ///
    /// # Panics
    ///
    /// Panics if the local component's path terminates with `..`.
    #[must_use]
    pub fn file_name(&self) -> PathBuf {
        match self {
            Self::Remote(remote_component) => remote_component.file_name.clone(),
            Self::Local(local_component) => local_component.path.file_name().unwrap().into(),
        }
    }

    #[must_use]
    pub const fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }

    #[must_use]
    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                Self::Remote(_) => "Remote",
                Self::Local(_) => "Local",
            }
        )
    }
}

/// A **remote** modpack component.
///
/// **Remote** here can be understood as **"downloadable from some kind of
/// remote API or server"**. This entity that represents components that you add
/// from [modrinth.com] or [curseforge.com], or perhaps other remote APIs.
///
/// [modrinth.com]: https://modrinth.com
/// [curseforge.com]: https://www.curseforge.com/minecraft
#[derive(Serialize, Deserialize, Clone, Debug)]
#[must_use]
pub struct RemoteComponent {
    pub download_url: Url,
    pub file_name: PathBuf,
    pub file_size: usize,
    pub version_id: String,
    pub hashes: Hashes,
}

/// A **local** modpack component.
///
/// **Local** here can be taken as "this component exists as an already existing
/// non-metadata file in the modpack's repository, and should be simply copied
/// as is to the resulting modpack".
///
/// To add a file (be it a mod, config file or whatnot), just place that file
/// into its relevant folder, and run `invar component import-local`. Invar will
/// recognize all non-metadata files as potential local components, and ask you
/// if those should be tracked. You can also manually populate the internal list
/// of local components, it is kept in `pack.yml`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[must_use]
pub struct LocalComponent {
    /// Path to the non-metadata file that should be included as-is.
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[must_use]
pub struct LocalComponentEntry {
    pub path: PathBuf,
    pub category: Category,
}

impl LocalComponentEntry {
    /// Returns the [`Id`] of this [`LocalComponentEntry`].
    ///
    /// # Panics
    ///
    /// Panics if the underlying path has no file stem.
    #[must_use]
    pub fn id(&self) -> Id {
        self.path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string()
            .into()
    }
}

/// Possible types (categories) of [`Component`]s.
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
    ValueEnum,
    Debug,
)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    #[serde(alias = "plugin" /* FIXME: this is a dirty fucking stub */)]
    Mod,
    Resourcepack,
    #[serde(alias = "shaderpack")]
    Shader,
    Datapack,
    Config,
}

/// Possible relations between a [`Component`] and its loading environment.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Requirement {
    #[serde(alias = "incompatible")]
    Unsupported,
    Optional,
    #[serde(other)]
    Required,
}

/// Client- and server-side requirements for a [`Component`].
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub client: Requirement,
    pub server: Requirement,
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match (self.client, self.server) {
            (
                Requirement::Required | Requirement::Optional,
                Requirement::Required | Requirement::Optional,
            ) => "client/server",
            (Requirement::Optional | Requirement::Required, Requirement::Unsupported) => "client",
            (Requirement::Unsupported, Requirement::Required | Requirement::Optional) => "server",
            (Requirement::Unsupported, Requirement::Unsupported) => "wtf",
        };
        write!(f, "{repr}")
    }
}

/// **SHA1** and **SHA256** hashes of a [`RemoteComponent`], combined.
#[serde_as]
#[must_use]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Hashes {
    pub sha1: Sha1,
    pub sha512: Sha512,
}

/// A thin wrapper around a [`serde`]-compatible **SHA1** hash.
#[serde_as]
#[must_use]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Sha1(#[serde_as(as = "serde_with::hex::Hex")] [u8; 20]);

/// A thin wrapper around a [`serde`]-compatible **SHA256** hash.
#[serde_as]
#[must_use]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Sha512(#[serde_as(as = "serde_with::hex::Hex")] [u8; 64]);

#[cfg(test)]
mod tests {
    pub const TEST_SHA1: &str = "cc297357ff0031f805a744ca3a1378a112c2ddf4";
    pub const TEST_SHA512: &str = "d0760a2df6f123fb3546080a85f3a44608e1f8ad9f9f7c57b5380cf72235ad380a5bbd494263639032d63bb0f0c9e0847a62426a6028a73a4b4c8e7734b4e8f5";

    mod component {
        use url::Url;

        use crate::{
            Category, Component, Env, Hashes, Id, RemoteComponent, Requirement, Sha1, Sha512,
            Source, Tag, TagInformation,
        };

        #[test]
        #[ignore = "Intentional explicit panick for stderr inspection"]
        fn serde() -> eyre::Result<()> {
            let download_url = "https://cdn.modrinth.com/data/LNytGWDc/versions/6R069CcK/create-1.20.1-0.5.1.j.jar";
            let remote_component = RemoteComponent {
                download_url: Url::parse(download_url)?,
                file_name: "create-1.20.1-0.5.1.j.jar".into(),
                file_size: 15_583_566,
                version_id: "6R069CcK".into(),
                hashes: Hashes {
                    sha1: Sha1([0; 20]),
                    sha512: Sha512([0; 64]),
                },
            };

            let component = Component {
                id: Id::from("create"),
                category: Category::Mod,
                tags: TagInformation {
                    main: Some(Tag::Technology),
                    others: vec![],
                },
                environment: Env {
                    client: Requirement::Required,
                    server: Requirement::Required,
                },
                source: Source::Remote(remote_component),
            };

            let yaml = serde_yml::to_string(&component)?;
            eprintln!("{yaml}");
            eyre::bail!("Panicked to inspect stderr. This test is ignored");
        }
    }

    mod hash {
        use super::{TEST_SHA1, TEST_SHA512};
        use crate::{Hashes, Sha1, Sha512};

        #[test]
        pub fn sha1_serde() {
            let yml = TEST_SHA1;
            let sha1: Sha1 = serde_yml::from_str(yml).unwrap();
            assert_eq!(serde_yml::to_string(&sha1).unwrap().trim(), yml);
        }

        #[test]
        pub fn sha512_serde() {
            let yml = TEST_SHA512;
            let sha1: Sha512 = serde_yml::from_str(yml).unwrap();
            assert_eq!(serde_yml::to_string(&sha1).unwrap().trim(), yml);
        }

        #[test]
        pub fn combined_serde() {
            let yml = format!("sha1: {TEST_SHA1}\nsha512: {TEST_SHA512}");
            let hashes: Hashes = serde_yml::from_str(&yml).unwrap();
            assert_eq!(serde_yml::to_string(&hashes).unwrap().trim(), yml);
        }
    }
}

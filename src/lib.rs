#![feature(never_type)]
#![feature(error_generic_member_access)]
#![feature(let_chains)]
#![doc = include_str!("../README.md")]

/// Main building blocks of this tool.
pub mod component;
pub use component::Component;

/// Modrinth's [**`.mrpack`** pack format](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack) implementation.
pub mod index;
pub use index::Index;

/// The Minecraft instance entity.
mod instance;
pub use instance::*;

/// Types and traits for interacting with persistent entities.
pub mod local_storage;

/// Top-level "modpack" entity.
mod pack;
pub use pack::*;

/// Interface for self-hosting a server with the pack.
pub mod server;

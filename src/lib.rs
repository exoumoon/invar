#![feature(never_type)]
#![feature(error_generic_member_access)]
#![doc = include_str!("../README.md")]

/// Main building blocks of this tool.
pub mod component;

/// Modrinth's [**`.mrpack`** pack format](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack) implementation.
pub mod index;

/// The Minecraft instance entity.
pub mod instance;

/// Types and traits for interacting with persistent entities.
pub mod local_storage;

/// Top-level "modpack" entity.
pub mod pack;

/// Interface for self-hosting a server with the pack.
pub mod server;

//! ## Local pack overrides
//!
//! The zip may also contain a directory named `overrides`. Files in this
//! directory will be copied to the root of the Minecraft Instance directory
//! upon installation by the launcher. For example:
//!
//! ```not-rust
//! my_modpack.mrpack/
//!     modrinth.index.json
//!     overrides/
//!         config/
//!             mymod.cfg
//!         options.txt
//! ```
//!
//! When installed, the contents of `overrides` will be copied to the Minecraft
//! Instance directory and end up similar to this:
//!
//! ```not-rust
//! .minecraft/
//!     config/
//!         mymod.cfg
//!     options.txt
//! ```
//!
//! ## Server overrides
//!
//! Along with the traditional `overrides` folder, Modrinth also has a server
//! overrides folder to eliminate the need for server packs.
//!
//! Server overrides work in a layer based approach. This means server overrides
//! folder (with the directory name `server-overrides`) will be applied after
//! the `overrides` folder, overwriting its contents. Here's an example:
//!
//! ```not-rust
//! my_modpack.mrpack/
//!     modrinth.index.json
//!     overrides/
//!         config/
//!             mymod.cfg
//!         options.txt
//!     server-overrides/
//!         config/
//!             mymod.cfg
//!             servermod.cfg
//! ```
//!
//! When installed, the contents of `overrides` will be copied to the Minecraft
//! Instance directory. Then the contents of the `server-overrides` will be
//! copied and end up similar to this:
//!
//! ```not-rust
//! .minecraft/
//!     config/
//!         mymod.cfg
//!         servermod.cfg
//!     options.txt
//! ```
//!
//!  
//! ## Client Overrides
//!
//! Modrinth also has a client overrides folder! The folders name is
//! `client-overrides`. It is functionally equivalent to the server overrides
//! folder (besides only being applied on the client), and works the same with
//! the layer based approach.
//!
//! Both the server and client override folders are optional.

use serde::Serialize;

pub const COMMON_OVERRIDES_FOLDER: &str = "overrides";
pub const SERVER_OVERRIDES_FOLDER: &str = "server-overrides";
pub const CLIENT_OVERRIDES_FOLDER: &str = "client-overrides";

#[derive(Debug, Default, Clone, Serialize)]
pub struct Overrides {}

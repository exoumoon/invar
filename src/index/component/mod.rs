use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod env;
mod hashes;
mod requirement;
pub use env::Env;
pub use hashes::Hashes;
pub use requirement::Requirement;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// The **runtime** path of this file, relative to the Minecraft instance directory.
    pub(crate) path: PathBuf,

    /// The hashes of the file specified. This **must** contain the SHA1 hash and the SHA512 hash.
    pub(crate) hashes: Hashes,

    /// For files that only exist on a specific environment, this field allows that to be specified.
    pub(crate) env: Env,

    /// An array containing HTTPS URLs where this file may be downloaded.
    pub(crate) downloads: Vec<Url>,

    /// An integer containing the size of the file, in bytes.
    pub(crate) file_size: usize,
}

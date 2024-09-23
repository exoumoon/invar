use super::Requirement;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub(crate) client: Requirement,
    pub(crate) server: Requirement,
}

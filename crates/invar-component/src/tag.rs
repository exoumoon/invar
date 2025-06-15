use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::Component;

/// Possible tags that can be associated with a [`Component`].
///
/// A [`Component`] would usually have a "main" tag and "other" tags.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Display, EnumIter)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[non_exhaustive]
pub enum Tag {
    /// An uncategorized tag added by the user.
    #[strum(to_string = "{0}")]
    Custom(String),

    Building,
    Combat,
    Compatibility,
    Dimensions,
    Farming,
    Gear,
    Library,
    Mobs,
    Overworld,
    Performance,
    Progression,
    Qol,
    Storage,
    Technology,
    Visual,
    Wildlife,
}

/// Helper struct to group together main and secondary tags.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct TagInformation {
    pub main: Option<Tag>,
    pub others: Vec<Tag>,
}

impl TagInformation {
    pub const fn none() -> Self {
        Self {
            main: None,
            others: vec![],
        }
    }
}

impl Default for TagInformation {
    fn default() -> Self {
        Self::none()
    }
}

/// An interface for interacting with tagged entities.
pub trait Tagged {
    fn tags(&self) -> &TagInformation;
    fn tags_mut(&mut self) -> &mut TagInformation;
}

impl Tagged for Component {
    fn tags(&self) -> &TagInformation {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut TagInformation {
        &mut self.tags
    }
}

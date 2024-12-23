use super::AddError;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

/// Possible tags that can be associated with a
/// [`Component`](crate::component::Component).
///
/// A component would usually have a "main" tag and "other" tags.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Display, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    /// Stuff that adds weapons and/or combat mechanics, like **Better
    /// Combat**.
    Combat,
    /// Stuff that adds compatibility between other components and/or
    /// [`Loader`](crate::instance::Loader)s.
    Compatibility,
    /// An uncategorized tag added by the user.
    #[strum(to_string = "{0}")]
    Custom(String),
    /// Stuff that adds new or modifies existing dimensions.
    Dimensions,
    /// Stuff that adds new food, crops and animals, like **Farmer's
    /// Delight**.
    Farming,
    /// Stuff that adds new weapons, tools and armor.
    Gear,
    /// Libraries for other components, like **Cloth Config API** or
    /// **Zeta**.
    Library,
    /// Stuff that adds new hostile mobs to the game, like **Born in
    /// Chaos**.
    Mobs,
    /// Overworld generation stuff, like **Tectonic** and **Geophilic**.
    Overworld,
    /// Stuff that improves the game's performance, like **Sodium**.
    Performance,
    /// Stuff that tweaks the game's progression, like **Improvable
    /// Skills**.
    Progression,
    /// Quality-of-Life components, like **Quark**.
    Qol,
    /// Stuff that expands the game's storage systems, like **Expanded
    /// Storage**.
    Storage,
    /// Stuff that introduces technology to the game, like **Create** or
    /// **AE2**.
    Technology,
    /// Stuff that improves the game's visuals, like **Euphoria Patches** or
    /// **Wakes**.
    Visual,
    /// Stuff that adds new wildlife to the game, like **Alex's Mobs**.
    Wildlife,
}

/// Helper struct to group together [`Component`](crate::component::Component)
/// tagging information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TagInformation {
    pub main: Option<Tag>,
    pub others: Vec<Tag>,
}

pub(super) fn pick_main_tag() -> Result<Option<Tag>, AddError> {
    let main_tag: Option<Tag> = {
        let message = "Choose the main tag for this component:";
        let options = Tag::iter()
            .filter(|tag| !matches!(tag, Tag::Custom(_)))
            .collect();
        match inquire::Select::new(message, options)
            .with_page_size(Tag::iter().count())
            .with_help_message("Skip with [Escape] to provide a custom tag")
            .prompt_skippable()?
        {
            tag @ Some(_) => tag,
            None => {
                let message = "Provide a custom tag for this component:";
                inquire::Text::new(message)
                    .prompt_skippable()?
                    .map(|tag| tag.trim().to_lowercase())
                    .map(Tag::Custom)
            }
        }
    };
    Ok(main_tag)
}

pub(super) fn pick_secondary_tags(main_tag: Option<&Tag>) -> Result<Vec<Tag>, AddError> {
    let other_tags: Vec<Tag> = {
        let message = "Add some additional tags for this component?";
        let options = Tag::iter()
            .filter(|tag| !matches!(tag, Tag::Custom(_)) && main_tag != Some(tag))
            .collect();
        inquire::MultiSelect::new(message, options)
            .with_page_size(Tag::iter().count())
            .with_help_message("This step can be freely skipped.")
            .prompt_skippable()?
            .unwrap_or_default()
    };
    Ok(other_tags)
}

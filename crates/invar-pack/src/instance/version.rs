use std::fmt;
use std::str::FromStr;

pub use semver;
use serde::{Deserialize, Serialize};

/// A [version of Minecraft], be it semantic one, a [`Snapshot`] or whatever.
///
/// # Note on possible edge cases
///
/// Turns out minecraft has a really weird versioning convention. You may be
/// tempted to think it's good old [semver], but oh boy it's not. There are
/// [`Snapshot`]s, shit like `22w13oneblockatatime` and `1.17` (sure, a valid
/// Minecraft version, but not a valid semantic one), `1.10-pre2` (same story),
/// and god knows what other edge cases that I haven't thought of. Those might
/// one day lead somebody to some frustration caused by Invar not recognizing a
/// minecraft version, but as I said [`Snapshot`]'s docs, I honestly can't be
/// fucked future-proofing for this kind of shit.
///
/// [version of Minecraft]: https://minecraft.wiki/w/Java_Edition_version_history
/// [semver]: https://semver.org
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(untagged)]
#[must_use]
pub enum MinecraftVersion {
    /// A regular minecraft semantic version, like `1.20.1` or `1.18.2-pre3`.
    Semantic(semver::Version),
    /// A minecraft snapshot, like [`18w10d`](https://minecraft.wiki/w/18w10d) or [`14w26a`](https://minecraft.wiki/w/14w26a).
    Snapshot(Snapshot),
    /// Some other minecraft version I have not prepared this thing for.
    Unknown(String),
}

impl<S> From<S> for MinecraftVersion
where
    S: AsRef<str>,
{
    fn from(value: S) -> Self {
        let str = value.as_ref();
        match semver::Version::from_str(str) {
            Ok(version) => Self::Semantic(version),
            Err(_) => {
                if let Ok(version) = semver::Version::from_str(&format!("{str}.0")) {
                    // HACK: This branch is supposed to let us parse versions like `1.17` into the
                    // [`Self::Semantic`] variant instead of [`Self::Unknown`], however this won't
                    // help in cases like `1.10-pre2`. Too bad.
                    Self::Semantic(version)
                } else if let Ok(snapshot) = Snapshot::from_str(str) {
                    Self::Snapshot(snapshot)
                } else {
                    Self::Unknown(str.to_string())
                }
            }
        }
    }
}

impl fmt::Display for MinecraftVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string_repr = match self {
            Self::Snapshot(snapshot) => snapshot.to_string(),
            Self::Unknown(string_ref) => string_ref.to_string(),
            Self::Semantic(version) => {
                let string_repr = version.to_string();
                match version.patch {
                    0 => string_repr.replacen(".0", "", 1),
                    _ => string_repr,
                }
            }
        };
        write!(f, "{string_repr}")?;
        Ok(())
    }
}

/// A [Minecraft snapshot](https://minecraft.wiki/w/Snapshot), like `18w10d`.
///
/// > **Warning:** There are snapshots like `22w13oneblockatatime`,
/// > `24w14potato` and some other weird ones which can't be parsed into this
/// > type, but I honestly can't be fucked handling edge cases like these...
/// > However, those can represented with a [`MinecraftVersion::Unknown`].
///
/// # [From the wiki](https://minecraft.wiki/w/Snapshot)
///
/// Snapshots use a unique naming format, unrelated to the `1.x` version
/// numbering used elsewhere. Snapshots use the format `YYwWWn`. `YY` is the
/// two-digit year, `w` simply stands for "week", `WW` is the two-digit week
/// number within the year, and n is a unique letter identifier – starting with
/// `a`, then `b`, and so on – which increments when there is more than one
/// release in a given week. For example, `18w10d` was the fourth snapshot (`d`)
/// released in the 10th week of 2018. Currently the highest letter reached is
/// `e`, a tie between `12w30e`, `13w47e` and `15w35e`. The naming convention is
/// only broken by `13w12~` and April Fools' snapshots.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[must_use]
pub struct Snapshot {
    pub year: u8,
    pub week: u8,
    pub identifier: char,
}

impl Snapshot {
    /// The length of a string-represented snapshot, in chars.
    pub const LENGTH: usize = "YYwWWn".len();

    /// A shorthand for creating a [`Snapshot`].
    pub const fn new(year: u8, week: u8, identifier: char) -> Self {
        Self {
            year,
            week,
            identifier,
        }
    }
}

/// Errors that may occur when parsing a [`Snapshot`] string.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SnapshotParseError {
    #[error("The snapshot string was {0} chars long, which is invalid")]
    WrongLength(usize),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl FromStr for Snapshot {
    type Err = SnapshotParseError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str.len() {
            Self::LENGTH => {
                let year: u8 = str[0..2].parse()?;
                let week: u8 = str[3..5].parse()?;
                let identifier = str.chars().last().unwrap();
                let snapshot = Self::new(year, week, identifier);
                Ok(snapshot)
            }
            wrong_length => Err(SnapshotParseError::WrongLength(wrong_length)),
        }
    }
}

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}w{:02}{}", self.year, self.week, self.identifier)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;
    use semver::Version as Semver;

    use super::{MinecraftVersion, Snapshot};

    #[rstest]
    #[case("24w01a", Snapshot::new(24, 1, 'a'))]
    #[case("18w10d", Snapshot::new(18, 10, 'd'))]
    #[case("15w35e", Snapshot::new(15, 35, 'e'))]
    #[case("14w26a", Snapshot::new(14, 26, 'a'))]
    fn snapshot_parsing(#[case] string_repr: &str, #[case] snapshot: Snapshot) {
        let _ = color_eyre::install();
        assert_eq!(string_repr, snapshot.to_string());
        assert_eq!(Ok(snapshot), Snapshot::from_str(string_repr));
    }

    #[rstest]
    #[case::semver("1.20.1", MinecraftVersion::Semantic(Semver::new(1, 20, 1)))]
    #[case::semver("1.12.2", MinecraftVersion::Semantic(Semver::new(1, 12, 2)))]
    #[case::semver("1.17", MinecraftVersion::Semantic(Semver::new(1, 17, 0)))]
    #[case::snapshot("24w01a", MinecraftVersion::Snapshot(Snapshot::new(24, 1, 'a')))]
    #[case::snapshot("18w10d", MinecraftVersion::Snapshot(Snapshot::new(18, 10, 'd')))]
    #[case::snapshot("15w35e", MinecraftVersion::Snapshot(Snapshot::new(15, 35, 'e')))]
    #[case::snapshot("14w26a", MinecraftVersion::Snapshot(Snapshot::new(14, 26, 'a')))]
    #[case::semver("1.21.2-rc2", MinecraftVersion::Semantic(Semver::parse("1.21.2-rc2").unwrap()))]
    #[case::semver("1.21.2-pre5", MinecraftVersion::Semantic(Semver::parse("1.21.2-pre5").unwrap()))]
    #[case::semver("1.5+mod", MinecraftVersion::Unknown(String::from("1.5+mod")))]
    fn version_parsing(#[case] string_repr: &str, #[case] version: MinecraftVersion) {
        let _ = color_eyre::install();
        assert_eq!(string_repr, version.to_string());
        assert_eq!(version, MinecraftVersion::from(string_repr));
    }
}

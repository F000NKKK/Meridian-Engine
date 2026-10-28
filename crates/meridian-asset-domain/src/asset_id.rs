//! [`AssetId`]: an asset's stable identity, and the `@asset_id:` kind.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;
use substrate_data::{Expected, RefId};

substrate_data::kind!(AssetKind = "asset_id": AssetId);

/// An asset's stable identity: 64 bits, written as 16 lower case hex digits
/// (`3f2a9c0e8b1d4a67`). It lives in the asset's `.meta` file, so it
/// survives renames and moves, and cooked content (`.mpak`) refers to
/// assets by it (`@asset_id:3f2a9c0e8b1d4a67`).
///
/// This is a name on disk, not a runtime handle: it carries no generation
/// and no lifetime. Turning it into a `resource` handle is the job of the
/// code that loads the asset. The data layer has no random source, so
/// whoever creates a `.meta` file chooses the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(u64);

/// How many hex digits an [`AssetId`] is written with.
const DIGITS: usize = 16;

impl AssetId {
    /// The id with these bits.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        AssetId(bits)
    }

    /// The id's bits.
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl FromStr for AssetId {
    type Err = AssetIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || AssetIdError {
            text: text.to_owned(),
        };
        let digits_only = text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if text.len() != DIGITS || !digits_only {
            return Err(bad());
        }
        u64::from_str_radix(text, 16)
            .map(AssetId)
            .map_err(|_| bad())
    }
}

impl NamedType for AssetId {
    const NAME: &'static str = "asset_id";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        stored.parse().ok()
    }
}

substrate_data::named_schema!(AssetId);

impl RefId for AssetId {
    const EXPECTED: Expected = Expected::Text;
    fn parse(spelled: &str) -> Option<Self> {
        spelled.parse().ok()
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

/// A text that is not an asset id.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{text}` is not an asset id (16 lower case hex digits)")]
pub struct AssetIdError {
    /// What was given.
    pub text: String,
}

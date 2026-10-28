//! Why a track was refused.

/// A keyframe list that cannot be a track.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClipError {
    /// A track needs at least one key.
    #[error("a track needs at least one keyframe")]
    NoKeys,
    /// The values do not match the keys for the interpolation mode.
    #[error("{values} values for {keys} keys do not fit the interpolation")]
    ValueCount {
        /// How many keys.
        keys: usize,
        /// How many values.
        values: usize,
    },
    /// Key times must be finite and strictly increasing.
    #[error("keyframe {index} is not later than the one before it")]
    TimesNotIncreasing {
        /// The offending key.
        index: usize,
    },
}

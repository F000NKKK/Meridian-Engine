//! The `.mmat` format version.

/// The `.mmat` format version this crate writes and reads. Fields are added
/// as optional with defaults; the version changes only for a breaking
/// change. A newer one is
/// [`MaterialError::UnsupportedFormat`](super::MaterialError::UnsupportedFormat).
pub const MATERIAL_FORMAT: u32 = 1;

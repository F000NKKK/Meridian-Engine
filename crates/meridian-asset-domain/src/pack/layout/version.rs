//! The envelope version.

/// The `.mpak` envelope version this crate writes. It reads this and every
/// older one (none yet); a newer one is [`PackError::UnsupportedVersion`](crate::PackError::UnsupportedVersion).
pub const PACK_VERSION: u16 = 1;

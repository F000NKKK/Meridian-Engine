//! The `.meta` format version.

/// The `.meta` format version this crate writes. It reads this one; an
/// older version would be read by migrating its value to this one first
/// (Substrate Data's evolution rules), never refused. A newer one is
/// [`MetaError::UnsupportedFormat`](super::MetaError::UnsupportedFormat).
pub const META_FORMAT: u32 = 1;

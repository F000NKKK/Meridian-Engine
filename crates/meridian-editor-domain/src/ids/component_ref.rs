//! [`ComponentRef`]: which component a message is about.

/// A component, by the name the host's `ComponentRegistry` knows it under —
/// the same typed name `.mscene` documents and `Describe` use, so there is
/// one id for a component everywhere.
pub type ComponentRef = meridian_data_substrate::ComponentName;

//! Why a skeleton was refused.

/// A skeleton that cannot be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SkeletonError {
    /// More joints than a skinned draw can address.
    #[error("{count} joints exceed the limit of {max}")]
    TooManyJoints {
        /// How many joints were given.
        count: usize,
        /// The limit, [`MAX_JOINTS`](super::MAX_JOINTS).
        max: usize,
    },
    /// A joint's parent does not come before it in the list.
    #[error("joint {joint} names parent {parent}, which does not come before it")]
    ParentNotBefore {
        /// The joint with the bad parent.
        joint: usize,
        /// The parent it names.
        parent: usize,
    },
}
